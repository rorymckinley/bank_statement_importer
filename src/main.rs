use csv::{Reader, StringRecord};
use dirs;
use std::fs;
use std::env;
use yaml_rust::{Yaml, YamlLoader};
use yaml_rust::emitter::YamlEmitter;
use yaml_rust::scanner::ScanError;
use chrono::{NaiveDate, Datelike};
use chrono::format::ParseError;
use bank_statement_importer::ui::UI;
use bank_statement_importer::raw_entry::{RawEntry, Direction};
use sha2::{Sha256, Digest};
use bank_statement_importer::{ActivityReport, CategorisedActivityReport, EntryType, CategorisedEntry, Sphere};
use bank_statement_importer::ui_choices::{PatternOverride, UiChoices};
use bank_statement_importer::entry_manager::{create_classification_from_choices, Classification, Pattern};
use bank_statement_importer::config::Config;

fn serialise(structure: &Yaml) -> String {
    let mut out = String::new();
    let mut emitter = YamlEmitter::new(&mut out);

    let _ = emitter.dump(structure);
    out
}

fn deserialise(contents: String) -> Result<Config, ScanError> {
    let mut config = YamlLoader::load_from_str(&contents)?;
    let contents = config.pop().unwrap().into_hash().unwrap();
    Ok(Config::new(contents))
}

fn get_date_boundaries(start_date_string: &str) -> Result<(NaiveDate, NaiveDate), ParseError> {
    println!("START DATE: {}", start_date_string);
    let start_date = NaiveDate::parse_from_str(start_date_string, "%Y%m%d")?;

    let (end_date_y, end_date_m) = if start_date.month() == 12 {
        (start_date.year() + 1, 1)
    } else {
        (start_date.year(), start_date.month() + 1)
    };

    let end_date = NaiveDate::from_ymd(end_date_y, end_date_m, 1);

    Ok((start_date, end_date))
}


fn main() {
    // new
    let mut config_path_new = dirs::home_dir().expect("Can't find home dir");
    config_path_new.push(".bank_statement_importer.yml.new");

    //new
    if config_path_new.exists() {
        println!("Config exists at {:?}", config_path_new);
    } else {
        fs::write(&config_path_new, serialise(&Config::template())).expect("Could not write new config file");
        println!("Config created at {:?}", config_path_new);
    }

    let mut new_config = deserialise(fs::read_to_string(&config_path_new).expect("Could not read new config file")).unwrap();

    let args : Vec<String>= env::args().collect();

    let input_directory_path = &args[1];
    let start_date_string = &args[2];

    let (start_date, end_date_excl) = get_date_boundaries(start_date_string).expect("Could not set date boundaries");

    let mut raw_entries: Vec<RawEntry> = Vec::new();

    for file in fs::read_dir(input_directory_path).unwrap() {
        let mut reader = Reader::from_path(file.unwrap().path()).unwrap();
        for result in reader.records() {
            let raw_entry = RawEntry::new(result.unwrap());
            if raw_entry.date >= start_date && raw_entry.date < end_date_excl {
                raw_entries.push(raw_entry);
            }
        }
    }

    let ui = UI {};
    let mut processed_fingerprints: Vec<String> = Vec::new();
    let mut report = ActivityReport { entries: Vec::new() };

    for entry in raw_entries {

        ui.display_entry(&entry);

        if processed_fingerprints.contains(&entry.fingerprint) {
            ui.display_duplicate();
            if ui.skip_duplicate() {
                ui.skipping_duplicate();
                continue;
            }
        }

        let mut choices = UiChoices {
            existing_pattern: None,
            pattern_override: None,
            category: None,
            transfer: None,
            sphere: None,
            create_pattern: None,
            snippet: None,
            require_confirmation: None
        };

        if let Some(p) = new_config.find_pattern(&entry) {
            choices.existing_pattern = Some(p);
            match p {
                Pattern::Outbound {
                    snippet: _, category: _, assign_as_expense: _, assign_as_personal, require_confirmation
                } => {
                    if *require_confirmation {
                        choices.pattern_override  = match ui.sphere_override(assign_as_personal) {
                                true => Some(PatternOverride { is_personal: !assign_as_personal }),
                                false => None,
                            }
                        };
                    },
                _ => ()
            }
        } else {
            ui.display_categories(&new_config.categories);
            choices.category = Some(ui.capture_category(&new_config.categories));
            choices.transfer = Some(ui.is_transfer());

            choices.sphere = match choices.transfer {
                Some(transfer) => {
                    if transfer {
                        Some(Sphere::Personal)
                    } else {
                        match entry.direction {
                            Direction::Outbound => {
                                match ui.is_personal() {
                                    true => Some(Sphere::Personal),
                                    false => Some(Sphere::Work)
                                }
                            },
                            Direction::Inbound => {
                                Some(Sphere::Personal)
                            }
                        }
                    }
                },
                None => None
            };

            choices.create_pattern = Some(ui.create_pattern());

            if let Some(create_pattern) = choices.create_pattern {
                if create_pattern {
                    choices.snippet = Some(ui.snippet());
                    choices.require_confirmation = match entry.direction {
                        Direction::Inbound => Some(false),
                        Direction::Outbound => {
                            if let Some(is_transfer) = choices.transfer {
                                match is_transfer {
                                    true => Some(false),
                                    false => {
                                        if let Some(sphere) = choices.sphere.clone() {
                                            let is_personal = match sphere {
                                                Sphere::Personal => true,
                                                Sphere::Work => false
                                            };
                                            Some(ui.require_confirmation(is_personal))
                                        } else {
                                            Some(false)
                                        }
                                    }
                                }
                            } else {
                                None
                            }
                        }
                    };
                }
            }

        }


        let classification = create_classification_from_choices(entry.direction, choices);

        let cat_entry = match classification {
            Classification::ExistingPattern(pattern, pattern_override_option) => {
                let sphere = match pattern_override_option {
                    Some(pattern_override) => {
                        if pattern_override.is_personal { Sphere::Personal } else { Sphere::Work }
                    },
                    None => {
                        match pattern {
                            Pattern::Inbound  { snippet, category, assign_as_income } => { Sphere::Personal },
                            Pattern::Outbound { snippet, category, assign_as_expense, assign_as_personal, require_confirmation } => {
                                if *assign_as_personal { Sphere::Personal } else { Sphere::Work }
                            }
                        }

                    }
                };
                match pattern {
                    Pattern::Inbound  { snippet, category, assign_as_income } => {
                        CategorisedEntry {
                            sphere,
                            category: category.clone(),
                            description: entry.description,
                            amount: entry.amount,
                            entry_type: if *assign_as_income { EntryType::Income } else { EntryType::Transfer },
                            date: entry.date
                        }
                    },
                    Pattern::Outbound { snippet, category, assign_as_expense, assign_as_personal, require_confirmation } => {
                        CategorisedEntry {
                            sphere,
                            category: category.clone(),
                            description: entry.description,
                            amount: entry.amount,
                            entry_type: if *assign_as_expense { EntryType::Expense } else { EntryType::Transfer },
                            date: entry.date
                        }
                    }
                }
            },
            Classification::NewPatternInbound { snippet, category, assign_as_income } => {
                if !new_config.categories.contains(&category) {
                    new_config.categories.push(category.clone());
                }

                let pattern = Pattern::Inbound {
                    snippet: snippet.clone(),
                    category: category.clone(),
                    assign_as_income: assign_as_income.clone()
                };

                new_config.inbound_patterns.push(pattern);

                CategorisedEntry {
                    sphere: Sphere::Personal,
                    category: category,
                    description: entry.description,
                    amount: entry.amount,
                    entry_type: if assign_as_income { EntryType::Income } else { EntryType::Transfer },
                    date: entry.date
                }
            },
            Classification::NewPatternOutbound { snippet, category, assign_as_expense, assign_as_personal, require_confirmation } => {
                if !new_config.categories.contains(&category) {
                    new_config.categories.push(category.clone());
                }

                let pattern = Pattern::Outbound {
                    snippet: snippet.clone(),
                    category: category.clone(),
                    assign_as_expense: assign_as_expense,
                    assign_as_personal: assign_as_personal,
                    require_confirmation: require_confirmation,
                };

                new_config.outbound_patterns.push(pattern);

                CategorisedEntry {
                    sphere: if assign_as_personal { Sphere::Personal } else { Sphere::Work },
                    category: category,
                    description: entry.description,
                    amount: entry.amount,
                    entry_type: if assign_as_expense { EntryType::Expense } else { EntryType::Transfer },
                    date: entry.date
                }
            },
            Classification::NoPatternInbound { category, assign_as_income } => {
                if !new_config.categories.contains(&category) {
                    new_config.categories.push(category.clone());
                }

                CategorisedEntry {
                    sphere: Sphere::Personal,
                    category: category,
                    description: entry.description,
                    amount: entry.amount,
                    entry_type: if assign_as_income { EntryType::Income } else { EntryType::Transfer },
                    date: entry.date
                }
            },
            Classification::NoPatternOutbound { category, assign_as_expense, assign_as_personal } => {
                if !new_config.categories.contains(&category) {
                    new_config.categories.push(category.clone());
                }

                CategorisedEntry {
                    sphere: if assign_as_personal { Sphere::Personal } else { Sphere::Work },
                    category: category,
                    description: entry.description,
                    amount: entry.amount,
                    entry_type: if assign_as_expense { EntryType::Expense } else { EntryType::Transfer },
                    date: entry.date
                }
            }
        };

        report.entries.push(cat_entry);

        // TODO Find  better way to do this
        processed_fingerprints.push(entry.fingerprint);

        fs::write(&config_path_new, serialise(&new_config.export())).expect("Could not write config file");
        new_config = deserialise(fs::read_to_string(&config_path_new).expect("Could not read config file")).expect("Could not parse config contents");
    }

    println!("");
    println!("REPORT");
    println!("");

    println!("Personal Expense: {}", report.total_personal());
    println!("Work Expense: {}", report.total_work());

    let mut cat_report = CategorisedActivityReport::new(&report, &new_config.categories);

    ui.display_categorised_report(&mut cat_report);
}
