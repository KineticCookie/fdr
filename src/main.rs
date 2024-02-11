use chrono::Local;
use clap::{Parser, Subcommand, ValueEnum};
use fdr;
use tokio;

#[derive(ValueEnum, Debug, Clone)]
enum SortMode {
    Original,
    Desc,
    Asc,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CLI {
    #[command(subcommand)]
    operation: Operation,

    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Debug, Subcommand, Clone)]
enum Operation {
    ShowNews {
        opml: String,
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        all: bool,
        #[arg(value_enum, default_value = "original")]
        sort: SortMode,
    },
    ShowSources {
        opml: String,
    },
}

async fn show_news(
    opml: &str,
    all: bool,
    sort: SortMode,
    now: chrono::DateTime<chrono::FixedOffset>,
) {
    let opml = fdr::read_opml(opml).unwrap();
    let mut previous_guids = Vec::<String>::new();
    // read seen from file
    let seen_file = "seen.txt";
    if let Ok(content) = std::fs::read_to_string(seen_file) {
        previous_guids = content.lines().map(|s| s.to_string()).collect();
    }
    let rss_outlines = fdr::get_rss_outlines(&opml);
    let mut all_items = Vec::<fdr::FeedItem>::new();
    for outline in rss_outlines {
        let channel = fdr::read_feed(&outline.xml_url).await.unwrap();
        let items = fdr::read_feed_items(&channel);
        all_items.extend(items);
    }
    match sort {
        SortMode::Original => {}
        SortMode::Desc => {
            all_items.sort_by(|a, b| b.pub_date.cmp(&a.pub_date));
        }
        SortMode::Asc => {
            all_items.sort_by(|a, b| a.pub_date.cmp(&b.pub_date));
        }
    }

    for item in all_items {
        let guid = item.get_id();
        let already_seen = previous_guids.iter().any(|g| *g == guid);
        if !already_seen || all {
            item.show(now, already_seen);
            previous_guids.push(guid.clone());
        }
    }
    std::fs::write(seen_file, previous_guids.join("\n")).unwrap();
}

fn show_sources(opml: String) {
    let opml = fdr::read_opml(&opml).unwrap();
    let rss_outlines = fdr::get_rss_outlines(&opml);
    for outline in rss_outlines {
        println!("{}", outline.title);
    }
}

#[tokio::main]
async fn main() {
    let now = Local::now().fixed_offset();
    let args = CLI::parse();
    match args.operation {
        Operation::ShowNews { opml, all, sort } => show_news(&opml, all, sort, now).await,
        Operation::ShowSources { opml } => show_sources(opml),
    }
}
