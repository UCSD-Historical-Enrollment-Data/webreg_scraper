use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about)]
#[command(propagate_version = true)]
pub struct CliArg {
    #[command(subcommand)]
    pub command: CliSubCmd,
}

#[derive(Subcommand)]
pub enum CliSubCmd {
    /// Creates a new API key that can be used for the scraper's WebReg API.
    #[clap(name = "create")]
    CreateKey {
        /// A description for the key, if any.
        #[clap(name = "desc", short, long)]
        desc: Option<String>,
    },
    /// Edits the description of an existing API key.
    #[clap(name = "editDesc")]
    EditDescription {
        /// The prefix of the API key you want to edit the description for.
        #[clap(name = "prefix", short, long)]
        prefix: String,
        /// A description to associate with the key.
        #[clap(name = "desc", short, long)]
        desc: Option<String>,
    },
    /// Deletes an API key from the database via its prefix.
    #[clap(name = "delete")]
    DeleteKey {
        /// The prefix of the API key to delete.
        #[clap(name = "prefix", short, long)]
        prefix: String,
    },
    /// Checks that the given API key is valid.
    #[clap(name = "check")]
    CheckKey {
        /// The prefix of the API key to check.
        #[clap(name = "prefix", short, long)]
        prefix: String,
        /// The token to validate against.
        #[clap(name = "token", short, long)]
        token: String,
    },
    /// Shows all current API keys.
    #[clap(name = "showAll")]
    ShowAll {
        /// Whether the tokens should be shown.
        #[clap(name = "showToken", short, long)]
        show_tokens: Option<bool>,
    },
}