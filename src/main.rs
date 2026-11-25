mod deck;
mod render;
mod tray;
use anyhow::{Context, Result, bail};
use helldeck::{
    Catalog,
    platform::{Config, bindings},
    ui::{Page, Panel, Tile},
};
const HELP: &str = "HellDeck — Helldivers 2 / Elgato MK.2\n  helldeck run [home-arrows|ctrl-wasd]   start paused\n  helldeck doctor                       inspect deck (stop run first)\n  helldeck catalog                      list ID, type and code\n  helldeck layout [missions]            print a panel\n  helldeck preview OUTPUT.png [missions] render a panel\n  helldeck dry-run ID [preset]          print keys; no input\n  helldeck recognize CROPPED_ROW.png    match four slots offline\nFirst Sync: focus game, press SYNC, drag tightly around your four selected icons.";
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    let cat = Catalog::load()?;
    match a.first().map(String::as_str) {
        None | Some("--help" | "-h" | "help") => println!("{HELP}\nBy {}", tray::CREDIT),
        Some("about" | "--version" | "-V") => println!(
            "HellDeck {}\nBy {}\nhttps://x.com/matthewrogers",
            env!("CARGO_PKG_VERSION"),
            tray::CREDIT
        ),
        Some("run") => deck::run(&cat, a.get(1).map(String::as_str))?,
        Some("doctor") => deck::doctor()?,
        Some("recognize") => {
            let path = a.get(1).context("expected cropped row PNG")?;
            let row = image::open(path)?.to_rgb8();
            let report = helldeck::sync::Matcher::local(&cat).scan(&row)?;
            for (slot, decision) in report.iter().enumerate() {
                decision.log(slot, &cat);
            }
            let matched = report.map(|s| s.accepted);
            let ids = matched
                .into_iter()
                .map(|x| x.map(|i| cat.entries[i].id.clone()))
                .collect::<Option<Vec<_>>>()
                .context("review: unrecognized or duplicate slot")?;
            println!("{}", ids.join(" "));
        }
        Some("catalog") => {
            for e in &cat.entries {
                println!("{}\t{}\t{}\t{}", e.id, e.group, e.code, e.name)
            }
        }
        Some("dry-run") => {
            let id = cat
                .id(a.get(1).context("expected stratagem ID")?)
                .context("unknown stratagem")?;
            println!(
                "{}",
                bindings(a.get(2).map_or("home-arrows", String::as_str))?
                    .describe(&cat.entries[id].code)?
            );
        }
        Some(kind @ ("preview" | "layout")) => {
            let cfg = Config::load(&cat)?;
            let mut p = Panel::new(cfg.slots.map(|id| id.and_then(|x| cat.id(&x))));
            if a.get(if kind == "layout" { 1 } else { 2 })
                .is_some_and(|s| s == "missions")
            {
                p.page = Page::Missions(0)
            }
            let tiles = p.tiles(&cat);
            if kind == "layout" {
                for row in tiles.chunks(5) {
                    println!(
                        "{}",
                        row.iter()
                            .map(|t| match t {
                                Tile::Blank => "—".into(),
                                Tile::Label(s) => s.clone(),
                                Tile::Icon(i) => cat.entries[*i].name.to_uppercase(),
                            })
                            .collect::<Vec<_>>()
                            .join(" | ")
                    )
                }
            } else {
                render::sheet(&tiles, &cat).save(a.get(1).context("expected output PNG path")?)?
            }
        }
        _ => bail!("{HELP}"),
    }
    Ok(())
}
