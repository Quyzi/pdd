use color_eyre::{Result, Section, eyre::eyre};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tokio::sync::broadcast::{self, Receiver};

pub mod arguments;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    InputFile(PathBuf),
    OutputFile(PathBuf),
    BlockSize(usize),
    BlockCount(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Argies {
    pub input_file: Option<PathBuf>,
    pub output_files: Vec<PathBuf>,
    pub block_size: usize,
    pub block_count: usize,
}

impl Default for Argies {
    fn default() -> Self {
        Self {
            input_file: None,
            output_files: vec![],
            block_size: 1024,
            block_count: 0,
        }
    }
}

impl Argument {
    pub fn parse() -> Result<Argies> {
        let mut args = Argies::default();
        for arg in std::env::args() {
            let Some((lhs, rhs)) = arg.split_once("=") else {
                continue;
            };
            let (lhs, rhs) = (lhs.to_ascii_lowercase(), rhs.to_ascii_lowercase());
            match lhs.as_str() {
                "if" => {
                    let path = PathBuf::from(rhs);
                    if !path.exists() {
                        return Err(eyre!("Input file does not exist")
                            .with_note(|| format!("input if={}", path.display().to_string())));
                    }
                    args.input_file = Some(path);
                }
                "of" => {
                    let path = PathBuf::from(rhs);
                    args.output_files.push(path);
                }
                "bs" => {
                    let size = rhs.parse::<usize>().map_err(|e| {
                        eyre!("Invalid block size")
                            .with_error(|| e)
                            .with_note(|| format!("input bs={rhs}"))
                    })?;
                    args.block_size = size;
                }
                "count" => {
                    let count = rhs.parse::<usize>().map_err(|e| {
                        eyre!("Invalid block count")
                            .with_error(|| e)
                            .with_note(|| format!("input count={rhs}"))
                    })?;
                    args.block_count = count;
                }
                _ => continue,
            }
        }
        if args.input_file.is_none() {
            return Err(eyre!("No input file given"));
        }
        Ok(args)
    }
}

pub struct OutFile {
    pub path: PathBuf,
    pub file: File,
    pub rx: Receiver<Vec<u8>>,
}

impl OutFile {
    pub fn new(path: &Path, rx: Receiver<Vec<u8>>) -> Result<Self> {
        let file = OpenOptions::new().create(true).write(true).open(path)?;
        Ok(Self {
            file,
            path: path.into(),
            rx,
        })
    }

    pub fn write_block(&mut self, block: Vec<u8>) {
        match self.file.write(&block) {
            Ok(n) => println!("wrote {n} bytes to {}", self.path.display()),
            Err(e) => eprintln!("failed to write block to {}: {e}", self.path.display()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Argument::parse()?;

    let (tx, _rx) = broadcast::channel::<Vec<u8>>(64);
    let input_file = args.input_file.unwrap();
    let mut input = OpenOptions::new().read(true).open(&input_file)?;
    for output_file in args.output_files {
        let rx = tx.subscribe();
        let file = OutFile::new(&output_file, rx)?;
        tokio::spawn(async move {
            let mut file = file;
            while let Ok(block) = file.rx.recv().await {
                file.write_block(block);
            }
        });
    }

    let mut buffer = vec![0u8; args.block_size];
    let mut count = 0;
    while let Ok(n) = input.read(&mut buffer) {
        if args.block_count > 0 && count >= args.block_count {
            break;
        }
        count = count.saturating_add(1);
        println!("Read {n} bytes from {}", input_file.display());
        tx.send(buffer)?;
        buffer = vec![0u8; args.block_size];
    }

    Ok(())
}
