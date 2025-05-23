use std::sync::Arc;

use clap::Args;
use colored::Colorize;
use prettytable::{format, Cell, Row, Table};
use serde_json::Value;
use sps_common::cache::Cache;
use sps_common::config::Config;
use sps_common::error::Result;
use sps_common::formulary::Formulary;
use sps_core::installed::{get_installed_packages, PackageType};
use sps_core::InstalledPackageInfo;

#[derive(Args, Debug)]
pub struct List {
    /// Show only formulas
    #[arg(long = "formula")]
    pub formula_only: bool,
    /// Show only casks
    #[arg(long = "cask")]
    pub cask_only: bool,
}

impl List {
    pub async fn run(&self, config: &Config, cache: Arc<Cache>) -> Result<()> {
        let installed = get_installed_packages(config).await?;
        // Only show the latest version for each name
        use std::collections::HashMap;
        let mut formula_map: HashMap<&str, &sps_core::installed::InstalledPackageInfo> =
            HashMap::new();
        let mut cask_map: HashMap<&str, &sps_core::installed::InstalledPackageInfo> =
            HashMap::new();
        for pkg in &installed {
            match pkg.pkg_type {
                PackageType::Formula => {
                    let entry = formula_map.entry(pkg.name.as_str()).or_insert(pkg);
                    if pkg.version > entry.version {
                        formula_map.insert(pkg.name.as_str(), pkg);
                    }
                }
                PackageType::Cask => {
                    let entry = cask_map.entry(pkg.name.as_str()).or_insert(pkg);
                    if pkg.version > entry.version {
                        cask_map.insert(pkg.name.as_str(), pkg);
                    }
                }
            }
        }
        let mut formulas: Vec<&InstalledPackageInfo> = formula_map.values().copied().collect();
        let mut casks: Vec<&InstalledPackageInfo> = cask_map.values().copied().collect();
        // Sort formulas and casks alphabetically by name, then version
        formulas.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
        casks.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
        // If Nothing Installed.
        if formulas.is_empty() && casks.is_empty() {
            println!("{}", "0 formulas and casks installed".yellow());
            return Ok(());
        }
        // If user wants to show installed formulas only.
        if self.formula_only {
            self.print_formulas_table(formulas, config);
            return Ok(());
        }
        // If user wants to show installed casks only.
        if self.cask_only {
            self.print_casks_table(casks, cache);
            return Ok(());
        }

        // Default Implementation
        let formulary = Formulary::new(config.clone());
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.add_row(Row::new(vec![
            Cell::new("Type").style_spec("b"),
            Cell::new("Name").style_spec("b"),
            Cell::new("Installed").style_spec("b"),
            Cell::new("New Version?").style_spec("b"),
        ]));
        let mut formula_count = 0;
        let mut cask_count = 0;
        for pkg in formulas {
            let latest = formulary.load_formula(&pkg.name).ok();
            let (has_new, _) = match latest {
                Some(ref f) => {
                    let latest_version = f.version_str_full();
                    (latest_version != pkg.version, latest_version)
                }
                None => (false, "-".to_string()),
            };
            table.add_row(Row::new(vec![
                Cell::new("Formula").style_spec("Fg"),
                Cell::new(&pkg.name).style_spec("Fb"),
                Cell::new(&pkg.version),
                // TODO: update to display the latest version string.
                // TODO: Not showing when the using --all flag.
                Cell::new(if has_new { "✔" } else { "" }),
            ]));
            formula_count += 1;
        }
        for pkg in casks {
            // Try to load cask info from cache
            let cask_val = cache.load_raw("cask.json").ok().and_then(|raw| {
                serde_json::from_str::<Vec<Value>>(&raw)
                    .ok()?
                    .into_iter()
                    .find(|v| v.get("token").and_then(|t| t.as_str()) == Some(&pkg.name))
            });
            let (has_new, _) = match cask_val {
                Some(ref v) => {
                    let latest_version = v.get("version").and_then(|v| v.as_str()).unwrap_or("-");
                    (latest_version != pkg.version, latest_version.to_string())
                }
                None => (false, "-".to_string()),
            };
            table.add_row(Row::new(vec![
                Cell::new("Cask").style_spec("Fy"),
                Cell::new(&pkg.name).style_spec("Fb"),
                Cell::new(&pkg.version),
                Cell::new(if has_new { "✔" } else { "" }),
            ]));
            cask_count += 1;
        }
        table.printstd();
        if formula_count > 0 && cask_count > 0 {
            println!(
                "{}",
                format!("{formula_count} formulas, {cask_count} casks installed").bold()
            );
        } else if formula_count > 0 {
            println!("{}", format!("{formula_count} formulas installed").bold());
        } else if cask_count > 0 {
            println!("{}", format!("{cask_count} casks installed").bold());
        }
        Ok(())
    }

    fn print_formulas_table(
        &self,
        formulas: Vec<&sps_core::installed::InstalledPackageInfo>,
        config: &Config,
    ) {
        if formulas.is_empty() {
            println!("No formulas installed.");
            return;
        }
        let formulary = Formulary::new(config.clone());
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        // Add header row with "Formulas" spanning all columns, font color green
        table.add_row(Row::new(vec![Cell::new_align(
            "Formulas",
            format::Alignment::CENTER,
        )
        .style_spec("bFg")
        .with_hspan(3)]));
        table.add_row(Row::new(vec![
            Cell::new("Name").style_spec("b"),
            Cell::new("Installed").style_spec("b"),
            Cell::new("New Version?").style_spec("b"),
        ]));
        let mut formula_count = 0;
        for pkg in formulas {
            let latest = formulary.load_formula(&pkg.name).ok();
            let (has_new, _) = match latest {
                Some(ref f) => {
                    let latest_version = f.version_str_full();
                    (latest_version != pkg.version, latest_version)
                }
                None => (false, "-".to_string()),
            };
            table.add_row(Row::new(vec![
                Cell::new(&pkg.name).style_spec("Fb"),
                Cell::new(&pkg.version),
                Cell::new(if has_new { "✔" } else { "" }),
            ]));
            formula_count += 1;
        }
        table.printstd();
        println!("{}", format!("{formula_count} formulas installed").bold());
    }

    fn print_casks_table(
        &self,
        casks: Vec<&sps_core::installed::InstalledPackageInfo>,
        cache: Arc<Cache>,
    ) {
        if casks.is_empty() {
            println!("No casks installed.");
            return;
        }
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        // Add header row with "Casks" spanning all columns, font color green
        table.add_row(Row::new(vec![Cell::new_align(
            "Casks",
            format::Alignment::CENTER,
        )
        .style_spec("bFg")
        .with_hspan(3)]));
        table.add_row(Row::new(vec![
            Cell::new("Name").style_spec("b"),
            Cell::new("Installed").style_spec("b"),
            Cell::new("New Version?").style_spec("b"),
        ]));
        let mut cask_count = 0;
        for pkg in casks {
            // Try to load cask info from cache
            let cask_val = cache.load_raw("cask.json").ok().and_then(|raw| {
                serde_json::from_str::<Vec<Value>>(&raw)
                    .ok()?
                    .into_iter()
                    .find(|v| v.get("token").and_then(|t| t.as_str()) == Some(&pkg.name))
            });
            let (has_new, _) = match cask_val {
                Some(ref v) => {
                    let latest_version = v.get("version").and_then(|v| v.as_str()).unwrap_or("-");
                    (latest_version != pkg.version, latest_version.to_string())
                }
                None => (false, "-".to_string()),
            };
            table.add_row(Row::new(vec![
                Cell::new(&pkg.name).style_spec("Fb"),
                Cell::new(&pkg.version),
                Cell::new(if has_new { "✔" } else { "" }),
            ]));
            cask_count += 1;
        }
        table.printstd();
        println!("{}", format!("{cask_count} casks installed").bold());
    }
}
