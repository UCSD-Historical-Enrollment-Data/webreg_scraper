mod types;

use crate::types::{CliArg, CliSubCmd};
use basicauth::{AuthCheckResult, AuthManager};
use clap::Parser;
use tabled::builder::Builder;
use tabled::settings::Style;

const AUTH_NAME: &str = "auth.db";

fn main() {
    let manager = AuthManager::new(AUTH_NAME);
    let args = CliArg::parse();
    match args.command {
        CliSubCmd::CreateKey { desc } => {
            println!("Description: {desc:?}");
            let key = manager.generate_api_key(desc);
            println!("✅ Generated API Key: {key}");
        }
        CliSubCmd::EditDescription { prefix, desc } => {
            println!("Prefix: {prefix}");
            println!("Description: {desc:?}");
            if manager.edit_description_by_prefix(prefix.as_str(), desc) {
                println!("✅ Edited Successfully!");
            } else {
                eprintln!("❌ Could not edit the description. Does the prefix exist?");
            }
        }
        CliSubCmd::DeleteKey { prefix } => {
            println!("Prefix: {prefix}");
            if manager.delete_by_prefix(prefix.as_str()) {
                println!("✅ Deleted Successfully!");
            } else {
                eprintln!("❌ Could not delete the key. Does it exist?");
            }
        }
        CliSubCmd::CheckKey { prefix, token } => {
            println!("Prefix: {prefix}");
            println!("Token: {token}");
            match manager.check_key(prefix.as_str(), token.as_str()) {
                AuthCheckResult::Valid => {
                    println!("✅ The key is valid!");
                }
                AuthCheckResult::NoPrefixOrTokenFound => {
                    println!("❌ The prefix or token is not found.");
                }
                AuthCheckResult::ExpiredKey => {
                    println!("❗ The key is found, but is expired.");
                }
            }
        }
        CliSubCmd::ShowAll { show_tokens } => {
            let mut table_builder = Builder::new();
            if show_tokens.unwrap_or(false) {
                table_builder.set_header(["Prefix", "Token", "Created", "Expired", "Description"]);
            } else {
                table_builder.set_header(["Prefix", "Created", "Expired", "Description"]);
            }

            let entries = manager.get_all_entries();
            println!("✅ Found {} API Keys.", entries.len());
            if !entries.is_empty() {
                for entry in entries {
                    let mut v = vec![];
                    v.push(entry.prefix);
                    if show_tokens.unwrap_or(false) {
                        v.push(entry.token);
                    }
                    v.push(entry.created_at.to_string());
                    v.push(entry.expires_at.to_string());
                    v.push(entry.description.unwrap_or("N/A".into()));
                    table_builder.push_record(v);
                }

                let mut table = table_builder.build();
                table.with(Style::rounded());
                println!("{table}");
            }
        }
    }
}
