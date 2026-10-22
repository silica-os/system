//! Shared EFI support for the Silica system tools: UEFI variables, boot
//! entries, GPT and ESP discovery. Standard library only.

pub mod args;
pub mod cmd;
pub mod crc32;
pub mod efivars;
pub mod error;
pub mod esp;
pub mod fixture;
pub mod gpt;
pub mod guid;
pub mod loadopt;
pub mod manager;
pub mod roots;
pub mod version;

pub use error::{Context, Error, Failure, Result};
