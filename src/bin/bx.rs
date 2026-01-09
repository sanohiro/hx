//! bx - Binary hex tool for pipes
//!
//! Unix-style binary manipulation tool.

use std::io::{self, Read, Write};

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};

/// Binary hex tool for pipes
#[derive(Parser, Debug)]
#[command(name = "bx")]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Find hex pattern in input, output matching offsets
    Find {
        /// Hex pattern to search (e.g., "DEADBEEF" or "DE AD BE EF")
        pattern: String,

        /// Input file (default: stdin)
        #[arg(short, long)]
        input: Option<String>,

        /// Output format: "hex" (default), "dec", "both"
        #[arg(short, long, default_value = "hex")]
        format: String,
    },

    /// Extract byte range from input
    Slice {
        /// Range in format "start:end" (hex with 0x prefix, or decimal)
        /// Examples: "0:100", "0x100:0x200", "100:"
        range: String,

        /// Input file (default: stdin)
        #[arg(short, long)]
        input: Option<String>,

        /// Output as hex dump instead of raw bytes
        #[arg(short = 'x', long)]
        hex: bool,
    },

    /// Replace hex pattern in input
    Replace {
        /// Pattern to find (hex)
        from: String,

        /// Pattern to replace with (hex)
        to: String,

        /// Input file (default: stdin)
        #[arg(short, long)]
        input: Option<String>,

        /// Replace all occurrences (default: first only)
        #[arg(short, long)]
        all: bool,
    },

    /// Patch bytes at specific offsets
    Patch {
        /// Patches in format "offset=hexvalue" (e.g., "0x100=FF" "0x200=DEAD")
        patches: Vec<String>,

        /// Input file (default: stdin)
        #[arg(short, long)]
        input: Option<String>,
    },

    /// Show file info (size, entropy, etc.)
    Info {
        /// Input file (default: stdin)
        #[arg(short, long)]
        input: Option<String>,
    },

    /// Convert between hex and binary
    Conv {
        /// Direction: "hex2bin" or "bin2hex"
        direction: String,

        /// Input file (default: stdin)
        #[arg(short, long)]
        input: Option<String>,

        /// For bin2hex: bytes per line (default: 16)
        #[arg(short, long, default_value = "16")]
        width: usize,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Find { pattern, input, format } => cmd_find(&pattern, input.as_deref(), &format),
        Command::Slice { range, input, hex } => cmd_slice(&range, input.as_deref(), hex),
        Command::Replace { from, to, input, all } => cmd_replace(&from, &to, input.as_deref(), all),
        Command::Patch { patches, input } => cmd_patch(&patches, input.as_deref()),
        Command::Info { input } => cmd_info(input.as_deref()),
        Command::Conv { direction, input, width } => cmd_conv(&direction, input.as_deref(), width),
    }
}

/// Read input from file or stdin
fn read_input(path: Option<&str>) -> Result<Vec<u8>> {
    match path {
        Some(p) => Ok(std::fs::read(p)?),
        None => {
            let mut buf = Vec::new();
            io::stdin().read_to_end(&mut buf)?;
            Ok(buf)
        }
    }
}

/// Parse hex string to bytes
fn parse_hex(s: &str) -> Result<Vec<u8>> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect();

    if cleaned.len() % 2 != 0 {
        bail!("Hex string must have even length");
    }

    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))
        })
        .collect()
}

/// Parse range string "start:end"
fn parse_range(s: &str, max_len: usize) -> Result<(usize, usize)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        bail!("Range must be in format 'start:end'");
    }

    let start = if parts[0].is_empty() {
        0
    } else {
        parse_offset(parts[0])?
    };

    let end = if parts[1].is_empty() {
        max_len
    } else {
        parse_offset(parts[1])?
    };

    Ok((start, end.min(max_len)))
}

/// Parse offset (hex with 0x prefix or decimal)
fn parse_offset(s: &str) -> Result<usize> {
    if s.starts_with("0x") || s.starts_with("0X") {
        usize::from_str_radix(&s[2..], 16).map_err(|e| anyhow::anyhow!("Invalid hex offset: {}", e))
    } else {
        s.parse().map_err(|e| anyhow::anyhow!("Invalid offset: {}", e))
    }
}

/// Find pattern in data
fn find_pattern(data: &[u8], pattern: &[u8]) -> Vec<usize> {
    let mut results = Vec::new();
    if pattern.is_empty() || pattern.len() > data.len() {
        return results;
    }

    for i in 0..=data.len() - pattern.len() {
        if &data[i..i + pattern.len()] == pattern {
            results.push(i);
        }
    }
    results
}

// === Commands ===

fn cmd_find(pattern: &str, input: Option<&str>, format: &str) -> Result<()> {
    let data = read_input(input)?;
    let pattern_bytes = parse_hex(pattern)?;
    let matches = find_pattern(&data, &pattern_bytes);

    for offset in matches {
        match format {
            "dec" => println!("{}", offset),
            "both" => println!("0x{:08X} ({})", offset, offset),
            _ => println!("0x{:08X}", offset),
        }
    }

    Ok(())
}

fn cmd_slice(range: &str, input: Option<&str>, hex_output: bool) -> Result<()> {
    let data = read_input(input)?;
    let (start, end) = parse_range(range, data.len())?;

    if start >= data.len() {
        bail!("Start offset {} exceeds file size {}", start, data.len());
    }

    let slice = &data[start..end];

    if hex_output {
        // Hex dump format
        for (i, chunk) in slice.chunks(16).enumerate() {
            let offset = start + i * 16;
            print!("{:08X}  ", offset);
            for (j, byte) in chunk.iter().enumerate() {
                print!("{:02X} ", byte);
                if j == 7 {
                    print!(" ");
                }
            }
            println!();
        }
    } else {
        // Raw binary output
        io::stdout().write_all(slice)?;
    }

    Ok(())
}

fn cmd_replace(from: &str, to: &str, input: Option<&str>, all: bool) -> Result<()> {
    let mut data = read_input(input)?;
    let from_bytes = parse_hex(from)?;
    let to_bytes = parse_hex(to)?;

    let matches = find_pattern(&data, &from_bytes);

    if matches.is_empty() {
        // No matches, output unchanged
        io::stdout().write_all(&data)?;
        return Ok(());
    }

    // Replace (from end to avoid offset shifts when replacing multiple)
    let indices: Vec<usize> = if all {
        matches.into_iter().rev().collect()
    } else {
        vec![matches[0]]
    };

    for offset in indices.iter().rev() {
        let end = offset + from_bytes.len();
        data.splice(*offset..end, to_bytes.iter().cloned());
    }

    io::stdout().write_all(&data)?;
    Ok(())
}

fn cmd_patch(patches: &[String], input: Option<&str>) -> Result<()> {
    let mut data = read_input(input)?;

    for patch in patches {
        let parts: Vec<&str> = patch.split('=').collect();
        if parts.len() != 2 {
            bail!("Patch must be in format 'offset=hexvalue': {}", patch);
        }

        let offset = parse_offset(parts[0])?;
        let value = parse_hex(parts[1])?;

        if offset + value.len() > data.len() {
            bail!("Patch at {} with {} bytes exceeds file size {}",
                  offset, value.len(), data.len());
        }

        data[offset..offset + value.len()].copy_from_slice(&value);
    }

    io::stdout().write_all(&data)?;
    Ok(())
}

fn cmd_info(input: Option<&str>) -> Result<()> {
    let data = read_input(input)?;

    println!("Size: {} bytes (0x{:X})", data.len(), data.len());

    if !data.is_empty() {
        // Entropy calculation
        let mut freq = [0u64; 256];
        for &byte in &data {
            freq[byte as usize] += 1;
        }
        let len = data.len() as f64;
        let entropy: f64 = freq.iter()
            .filter(|&&f| f > 0)
            .map(|&f| {
                let p = f as f64 / len;
                -p * p.log2()
            })
            .sum();
        println!("Entropy: {:.4} bits/byte", entropy);

        // Null byte percentage
        let nulls = freq[0];
        println!("Null bytes: {} ({:.1}%)", nulls, nulls as f64 / len * 100.0);

        // Printable ASCII percentage
        let printable: u64 = (0x20u8..=0x7E).map(|b| freq[b as usize]).sum();
        println!("Printable ASCII: {} ({:.1}%)", printable, printable as f64 / len * 100.0);
    }

    Ok(())
}

fn cmd_conv(direction: &str, input: Option<&str>, width: usize) -> Result<()> {
    match direction {
        "bin2hex" | "b2h" => {
            let data = read_input(input)?;
            for chunk in data.chunks(width) {
                for byte in chunk {
                    print!("{:02X} ", byte);
                }
                println!();
            }
        }
        "hex2bin" | "h2b" => {
            let mut text = String::new();
            match input {
                Some(p) => {
                    text = std::fs::read_to_string(p)?;
                }
                None => {
                    io::stdin().read_to_string(&mut text)?;
                }
            }
            let bytes = parse_hex(&text)?;
            io::stdout().write_all(&bytes)?;
        }
        _ => bail!("Direction must be 'bin2hex' (b2h) or 'hex2bin' (h2b)"),
    }
    Ok(())
}
