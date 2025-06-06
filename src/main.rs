use anyhow::{Result, anyhow};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    InputFile(PathBuf),
    OutputFile(PathBuf),
    BlockSize(usize),
    BlockCount(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arguments {
    pub input_file: PathBuf,
    pub output_files: Vec<PathBuf>,
    pub block_size: usize,
    pub block_count: usize,
}

impl Default for Arguments {
    fn default() -> Self {
        Self {
            input_file: PathBuf::from("."),
            output_files: vec![],
            block_size: 1024,
            block_count: 0,
        }
    }
}

impl Argument {
    pub fn parse() -> Result<Arguments> {
        let mut args = Arguments::default();
        for arg in std::env::args() {
            let Some((lhs, rhs)) = arg.split_once("=") else {
                continue;
            };
            match lhs.to_ascii_lowercase().as_str() {
                "if" => {
                    let path = PathBuf::from(rhs);
                    if !path.exists() {
                        return Err(anyhow!("Input file {} does not exist", path.display()));
                    }
                    args.input_file = path;
                }
                "of" => {
                    let path = PathBuf::from(rhs);
                    args.output_files.push(path);
                }
                "bs" => {
                    let size = rhs
                        .parse::<usize>()
                        .map_err(|e| anyhow!("Invalid block size: {}", e))?;
                    args.block_size = size;
                }
                "count" => {
                    let count = rhs
                        .parse::<usize>()
                        .map_err(|e| anyhow!("Invalid block count: {}", e))?;
                    args.block_count = count;
                }
                _ => continue,
            }
        }
        Ok(args)
    }
}

fn main() -> Result<()> {
    let args = Argument::parse()?;

    let mut input_file = OpenOptions::new().read(true).open(&args.input_file)?;
    let mut output_files = vec![];
    for output_file in args.output_files {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&output_file)?;
        output_files.push((output_file, file));
    }

    let mut buffer = vec![0; args.block_size];
    let mut count = 0;
    while let Ok(n) = input_file.read(&mut buffer) {
        if args.block_count > 0 && count >= args.block_count {
            break;
        }
        for (path, output_file) in &mut output_files {
            match output_file.write_all(&buffer[..n]) {
                Ok(_) => println!("Wrote {} bytes to {}", n, path.display()),
                Err(e) => {
                    return Err(anyhow!(
                        "Error writing to output file {}: {}",
                        path.display(),
                        e
                    ));
                }
            }
        }
        count += 1;
    }

    Ok(())
}
