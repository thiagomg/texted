use std::fmt::{Display, Formatter};

use clap::{arg, Parser, ValueEnum};

use crate::bootstrap::bootstrap_cmd;
use crate::post::post_cmd;

mod test_data;
mod decompress;
mod post;
mod bootstrap;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
enum Args {
    /// Creating post
    Post(PostArgs),
    /// Bootstrap a new blog
    Bootstrap(BootstrapArgs),
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct PostArgs {
    /// Name of the author. If empty, OS user real name is being used
    #[arg(short, long)]
    name: Option<String>,

    /// Title of the post
    #[arg(short, long)]
    title: Option<String>,

    /// Post generation options
    #[arg(short, long, default_value_t = PostOutput::Stdout)]
    output: PostOutput,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct BootstrapArgs {
    /// Directory where the new blog will be generated
    #[arg(short, long)]
    out_dir: String,
}


#[derive(Clone, Debug, ValueEnum)]
enum PostOutput {
    /// Writes the new post content to the stdout
    Stdout,
    /// Writes the new post content to a file (posts without images)
    File,
    /// Writes the new post content to a directory (posts with images)
    Dir,
}

impl Display for PostOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

fn main() {
    let args = Args::parse();

    match args {
        Args::Post(args) => post_cmd(args),
        Args::Bootstrap(args) => bootstrap_cmd(args),
    };
}

