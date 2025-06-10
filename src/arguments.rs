use color_eyre::{Result, eyre::eyre};
use std::{path::PathBuf, str::FromStr};

// pdd if=boot.img of=/dev/sda1 of=/dev/sdb1 of=/dev/sdc1 \
//  -- if=root.img of=/dev/sda2 of=/dev/sdb2 of=/dev/sdc2 \
//  -- if=var.img of=/dev/sda3 of=/dev/sdb3 of=/dev/sdc3 \
//  -- if=stdout.log os=localhost:9000 redir=1 \
//  -- if=stderr.log os=localhost:9001 redir=1 \
//  1>stdout.log 2>stderr.log

const SEPARATOR: &'static str = "--";

#[derive(Clone, Default)]
pub struct Arguments {
    pub operations: Vec<Operation>,
}

#[derive(Clone)]
pub struct Operation {
    /// Path to the input file
    pub input_file: PathBuf,

    /// Paths to the outputs
    pub outputs: Vec<Output>,

    /// Block size
    ///
    /// (default = 1024)
    pub block_size: u64,

    /// Number of blocks
    ///
    /// (default = 0|ALL)
    pub count: u64,

    /// True if the input file is redirected output, e.g. stdout.
    ///
    /// (default = false)
    pub is_redirected: bool,
}

#[derive(Clone)]
pub enum Output {
    File(PathBuf),
    Socket(String, u16),
    Http { method: String, url: String },
}

#[derive(Clone)]
pub struct OperationBuilder {
    pub input_file: Option<PathBuf>,
    pub outputs: Vec<Output>,
    pub is_redirected: bool,
    pub block_size: u64,
    pub count: u64,
}

impl Default for OperationBuilder {
    fn default() -> Self {
        Self {
            input_file: None,
            outputs: vec![],
            is_redirected: false,
            block_size: 1024,
            count: 0,
        }
    }
}

impl OperationBuilder {
    pub fn input_file(&mut self, path: PathBuf) {
        let _ = self.input_file.replace(path);
    }

    pub fn output_file(&mut self, path: PathBuf) {
        self.outputs.push(Output::File(path))
    }

    pub fn output_socket(&mut self, hostname: &str, port: u16) {
        self.outputs
            .push(Output::Socket(hostname.to_string(), port))
    }

    pub fn output_http(&mut self, method: &str, url: &str) {
        self.outputs.push(Output::Http {
            method: method.to_string(),
            url: url.to_string(),
        })
    }

    pub fn block_size(&mut self, bs: u64) {
        self.block_size = bs
    }

    pub fn count(&mut self, c: u64) {
        self.count = c
    }

    pub fn is_redirected(&mut self) {
        self.is_redirected = !self.is_redirected;
    }

    pub fn build(self) -> Result<Operation> {
        // There must be an input file
        let Some(input_file) = self.input_file else {
            return Err(eyre!("Operation is missing input file"));
        };

        if self.outputs.is_empty() {
            return Err(eyre!("Operation must have at least one output"));
        }

        Ok(Operation {
            input_file: input_file.clone(),
            outputs: self.outputs,
            block_size: self.block_size,
            is_redirected: self.is_redirected,
            count: self.count,
        })
    }
}

impl Arguments {
    pub fn parse() -> Result<Self> {
        let mut args = Self::default();
        let mut op = OperationBuilder::default();
        for arg in std::env::args() {
            if arg == SEPARATOR {
                let this = std::mem::take(&mut op).build()?;
                args.operations.push(this);
                continue;
            }

            let Some((lhs, rhs)) = arg.split_once('=') else {
                return Err(eyre!(
                    "Invalid command line argument, expected key=value pair, got {arg}"
                ));
            };
            let (lhs, rhs) = (lhs.trim(), rhs.trim());
            match lhs {
                "if" => op.input_file(PathBuf::from_str(rhs)?),
                "of" => op.output_file(PathBuf::from_str(rhs)?),
                "os" => {
                    let Some((mut hostname, port_str)) = rhs.split_once(':') else {
                        return Err(eyre!(
                            "Invalid command line argument, expected os=hostname:port, got {arg}"
                        ));
                    };
                    let port = port_str.parse()?;
                    if hostname.is_empty() {
                        hostname = "localhost";
                    }
                    op.output_socket(hostname, port);
                }
                "ohttp" => {
                    let Some((method, url)) = rhs.split_once(';') else {
                        return Err(eyre!(
                            "Invalid command line argument, expected ohttp=[METHOD];[URL], got {rhs}"
                        ));
                    };
                }
                "bs" => {
                    let block_size: u64 = rhs.parse()?;
                    op.block_size(block_size);
                }
                "count" | "c" => {
                    let count: u64 = rhs.parse()?;
                    op.count(count);
                }
                "redir" => op.is_redirected(),
                _ => {
                    return Err(eyre!(
                        "Invalid command line argument, unexpected input {arg}"
                    ));
                }
            }
        }
        if op.input_file.is_some() {
            args.operations.push(op.build()?);
        }

        Ok(args)
    }
}
