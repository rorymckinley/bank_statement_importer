use csv::{Reader, StringRecord};
use dirs;
use std::fs;
use std::env;
use std::process::exit;
use std::collections::HashMap;
use yaml_rust::{Yaml, YamlLoader};
use yaml_rust::emitter::YamlEmitter;
use yaml_rust::scanner::ScanError;
use linked_hash_map::LinkedHashMap;
use chrono::{NaiveDate, Datelike};
use chrono::format::ParseError;
use bank_statement_importer::ui::UI;
use bank_statement_importer::raw_entry::{RawEntry, Direction};
use rust_decimal::Decimal;
use std::str::FromStr;
use sha2::{Sha256, Digest};
use bank_statement_importer::{ActivityReport, CategorisedActivityReport, EntryType, CategorisedEntry, Sphere};


#[test]
fn test_creating_config_template() {
    let mut expected: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
    expected.insert(Yaml::from_str("categories"), Yaml::Array(Vec::new()));
    expected.insert(Yaml::from_str("inbound_patterns"), Yaml::Array(Vec::new()));
    expected.insert(Yaml::from_str("outbound_patterns"), Yaml::Array(Vec::new()));

    assert_eq!(Yaml::Hash(expected), NewConfig::template());
}

#[derive(Debug)]
#[derive(PartialEq)]
enum Pattern {
    Inbound { snippet: String, category: String, assign_as_income: bool },
    Outbound { snippet: String, category: String, assign_as_expense: bool, assign_as_personal: bool, require_confirmation: bool }
}

#[test]
fn test_instantiating_from_yaml() {
    let mut config_input: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
    let categories = Yaml::Array(vec![Yaml::from_str("foo"), Yaml::from_str("bar")]);
    let mut outbound_patterns =  Vec::new();
    let mut inbound_patterns =  Vec::new();

    let mut pattern_one: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
    pattern_one.insert(Yaml::from_str("snippet"), Yaml::from_str("OnE"));
    pattern_one.insert(Yaml::from_str("category"), Yaml::from_str("foo"));
    pattern_one.insert(Yaml::from_str("assign_as_income"), Yaml::Boolean(true));
    pattern_one.insert(Yaml::from_str("require_confirmation"), Yaml::Boolean(false));
    let mut pattern_two: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
    pattern_two.insert(Yaml::from_str("snippet"), Yaml::from_str("tWo"));
    pattern_two.insert(Yaml::from_str("category"), Yaml::from_str("bar"));
    pattern_two.insert(Yaml::from_str("assign_as_income"), Yaml::Boolean(false));
    pattern_two.insert(Yaml::from_str("require_confirmation"), Yaml::Boolean(true));

    inbound_patterns.push(Yaml::Hash(pattern_one));
    inbound_patterns.push(Yaml::Hash(pattern_two));

    let mut pattern_three: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
    pattern_three.insert(Yaml::from_str("snippet"), Yaml::from_str("Three"));
    pattern_three.insert(Yaml::from_str("category"), Yaml::from_str("baz"));
    pattern_three.insert(Yaml::from_str("assign_as_expense"), Yaml::Boolean(true));
    pattern_three.insert(Yaml::from_str("assign_as_personal"), Yaml::Boolean(true));
    pattern_three.insert(Yaml::from_str("require_confirmation"), Yaml::Boolean(false));
    let mut pattern_four: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
    pattern_four.insert(Yaml::from_str("snippet"), Yaml::from_str("Four"));
    pattern_four.insert(Yaml::from_str("category"), Yaml::from_str("buzz"));
    pattern_four.insert(Yaml::from_str("assign_as_expense"), Yaml::Boolean(false));
    pattern_four.insert(Yaml::from_str("assign_as_personal"), Yaml::Boolean(false));
    pattern_four.insert(Yaml::from_str("require_confirmation"), Yaml::Boolean(true));

    outbound_patterns.push(Yaml::Hash(pattern_three));
    outbound_patterns.push(Yaml::Hash(pattern_four));

    config_input.insert(Yaml::from_str("categories"), categories);
    config_input.insert(Yaml::from_str("outbound_patterns"), Yaml::Array(outbound_patterns));
    config_input.insert(Yaml::from_str("inbound_patterns"), Yaml::Array(inbound_patterns));

    let config = NewConfig::new(config_input);

    assert_eq!(config.categories, vec!("foo", "bar"));
    assert_eq!(
        config.inbound_patterns,
        vec![
            Pattern::Inbound { snippet: String::from("OnE"), category: String::from("foo"), assign_as_income: true },
            Pattern::Inbound { snippet: String::from("tWo"), category: String::from("bar"), assign_as_income: false },
        ]
        );
    assert_eq!(
        config.outbound_patterns,
        vec![
            Pattern::Outbound { snippet: String::from("Three"), category: String::from("baz"), assign_as_expense: true, assign_as_personal: true, require_confirmation: false },
            Pattern::Outbound { snippet: String::from("Four"), category: String::from("buzz"), assign_as_expense: false, assign_as_personal: false, require_confirmation: true },
        ]
        );
}

#[test]
fn test_matching_patterns() {
    let pattern_one = Pattern::Inbound {
        snippet: String::from("one"), category: String::from("foo"), assign_as_income: true
    };
    let pattern_two = Pattern::Inbound {
        snippet: String::from("two"), category: String::from("bar"), assign_as_income: false
    };
    let pattern_three = Pattern::Outbound {
        snippet: String::from("three"), category: String::from("baz"), assign_as_expense: true,
        assign_as_personal: true, require_confirmation: false
    };
    let pattern_four = Pattern::Outbound {
        snippet: String::from("four"), category: String::from("buzz"), assign_as_expense: false,
        assign_as_personal: false, require_confirmation: true
    };

    let config = NewConfig {
        categories: vec![],
        inbound_patterns: vec![pattern_one, pattern_two],
        outbound_patterns: vec![pattern_three, pattern_four]
    };

    let outbound_entry = RawEntry::new(
        StringRecord::from(vec!["  20191101  ", "  foo three bar  ", "  -191.60  ", "  200.00  "])
        );
    let outbound_entry_no_match = RawEntry::new(
        StringRecord::from(vec!["  20191101  ", "  foo zzz bar  ", "  -191.60  ", "  200.00  "])
        );

    let inbound_entry = RawEntry::new(
        StringRecord::from(vec!["  20191101  ", "  foo one bar  ", "  191.60  ", "  200.00  "])
        );
    let inbound_entry_no_match = RawEntry::new(
        StringRecord::from(vec!["  20191101  ", "  foo zzz bar  ", "  191.60  ", "  200.00  "])
        );

    assert_eq!(config.find_pattern(&outbound_entry_no_match), None);
    assert_eq!(
        config.find_pattern(&inbound_entry),
        Some(
            &Pattern::Inbound {
                snippet: String::from("one"), category: String::from("foo"), assign_as_income: true,
            }
            )
        );
    assert_eq!(
        config.find_pattern(&outbound_entry),
        Some(
            &Pattern::Outbound {
                snippet: String::from("three"), category: String::from("baz"), assign_as_expense: true,
                assign_as_personal: true, require_confirmation: false
            }
            )
        )
}


#[derive(Debug)]
struct NewConfig {
    categories: Vec<String>,
    inbound_patterns: Vec<Pattern>,
    outbound_patterns: Vec<Pattern>
}

impl NewConfig {
    fn new(contents: LinkedHashMap<Yaml, Yaml>) -> NewConfig {
        let categories: Vec<String> = contents[&Yaml::from_str("categories")].as_vec().unwrap().iter().map(|x| String::from(x.as_str().unwrap())).collect();
        let inbound_patterns: Vec<Pattern> = contents[&Yaml::from_str("inbound_patterns")].
            as_vec().unwrap().iter().
            map(|x| x.as_hash().unwrap()).
            map(|x| Pattern::Inbound {snippet: String::from(x[&Yaml::from_str("snippet")].as_str().unwrap()), category: String::from(x[&Yaml::from_str("category")].as_str().unwrap()), assign_as_income: x[&Yaml::from_str("assign_as_income")].as_bool().unwrap()}).
            collect();

        let outbound_patterns: Vec<Pattern> = contents[&Yaml::from_str("outbound_patterns")].
            as_vec().unwrap().iter().
            map(|x| x.as_hash().unwrap()).
            map(|x| Pattern::Outbound {snippet: String::from(x[&Yaml::from_str("snippet")].as_str().unwrap()), category: String::from(x[&Yaml::from_str("category")].as_str().unwrap()), assign_as_expense: x[&Yaml::from_str("assign_as_expense")].as_bool().unwrap(), assign_as_personal: x[&Yaml::from_str("assign_as_personal")].as_bool().unwrap(), require_confirmation: x[&Yaml::from_str("require_confirmation")].as_bool().unwrap()}).
            collect();
        NewConfig {
            categories: categories,
            inbound_patterns: inbound_patterns,
            outbound_patterns: outbound_patterns
        }
    }

    fn find_pattern(&self, entry: &RawEntry) -> Option<&Pattern> {
        let patterns = match entry.direction {
            Direction::Outbound => {
                &self.outbound_patterns
            },
            Direction::Inbound => {
                &self.inbound_patterns
            }
            _ => &self.outbound_patterns
        };
        let mut matched_patterns = patterns.iter().filter(|p| {
            let snippet = match  p {
                Pattern::Outbound {category: _, assign_as_expense: _, assign_as_personal: _, require_confirmation: _, snippet: snip} => snip,
                Pattern::Inbound {category: _, assign_as_income: _,  snippet: snip} => snip,
            };
            entry.description.contains(snippet)
        });
        matched_patterns.next()
    }

    fn export(&self) -> Yaml {
        let mut config: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
        let categories_yaml: Vec<Yaml> = self.categories.iter().map(|c| Yaml::from_str(c)).collect();

        let inbound_patterns: Vec<Yaml> = self.inbound_patterns.iter().map(|p| {
            match p {
                Pattern::Inbound { snippet, category, assign_as_income } => {
                    let mut as_hash: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
                    as_hash.insert(Yaml::from_str("snippet"), Yaml::from_str(snippet));
                    as_hash.insert(Yaml::from_str("category"), Yaml::from_str(category));
                    as_hash.insert(Yaml::from_str("assign_as_income"), Yaml::Boolean(*assign_as_income));
                    Yaml::Hash(as_hash)
                },
                Pattern::Outbound { snippet, category, assign_as_expense, assign_as_personal, require_confirmation } => {
                    let mut as_hash: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
                    as_hash.insert(Yaml::from_str("snippet"), Yaml::from_str(snippet));
                    as_hash.insert(Yaml::from_str("category"), Yaml::from_str(category));
                    as_hash.insert(Yaml::from_str("assign_as_expense"), Yaml::Boolean(*assign_as_expense));
                    as_hash.insert(Yaml::from_str("assign_as_personal"), Yaml::Boolean(*assign_as_personal));
                    as_hash.insert(Yaml::from_str("require_confirmation"), Yaml::Boolean(*require_confirmation));
                    Yaml::Hash(as_hash)
                }
            }
        }).collect();

        let outbound_patterns: Vec<Yaml> = self.outbound_patterns.iter().map(|p| {
            match p {
                Pattern::Inbound { snippet, category, assign_as_income } => {
                    let mut as_hash: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
                    as_hash.insert(Yaml::from_str("snippet"), Yaml::from_str(snippet));
                    as_hash.insert(Yaml::from_str("category"), Yaml::from_str(category));
                    as_hash.insert(Yaml::from_str("assign_as_income"), Yaml::Boolean(*assign_as_income));
                    Yaml::Hash(as_hash)
                },
                Pattern::Outbound { snippet, category, assign_as_expense, assign_as_personal, require_confirmation } => {
                    let mut as_hash: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
                    as_hash.insert(Yaml::from_str("snippet"), Yaml::from_str(snippet));
                    as_hash.insert(Yaml::from_str("category"), Yaml::from_str(category));
                    as_hash.insert(Yaml::from_str("assign_as_expense"), Yaml::Boolean(*assign_as_expense));
                    as_hash.insert(Yaml::from_str("assign_as_personal"), Yaml::Boolean(*assign_as_personal));
                    as_hash.insert(Yaml::from_str("require_confirmation"), Yaml::Boolean(*require_confirmation));
                    Yaml::Hash(as_hash)
                }
            }
        }).collect();

        config.insert(Yaml::from_str("categories"), Yaml::Array(categories_yaml));
        config.insert(Yaml::from_str("inbound_patterns"), Yaml::Array(inbound_patterns));
        config.insert(Yaml::from_str("outbound_patterns"), Yaml::Array(outbound_patterns));

        Yaml::Hash(config)
    }

    fn template() -> Yaml {
        let mut config: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
        config.insert(Yaml::from_str("categories"), Yaml::Array(Vec::new()));
        config.insert(Yaml::from_str("inbound_patterns"), Yaml::Array(Vec::new()));
        config.insert(Yaml::from_str("outbound_patterns"), Yaml::Array(Vec::new()));
        Yaml::Hash(config)
    }
}

#[derive(Debug)]
struct PatternOverride {
    is_personal: bool
}

#[ derive(Debug) ]
enum Classification<'a> {
    ExistingPattern(&'a Pattern, Option<PatternOverride>),
    NewPatternInbound {snippet: String, category: String, assign_as_income: bool},
    NewPatternOutbound {snippet: String, category: String, assign_as_expense: bool, assign_as_personal: bool, require_confirmation: bool },
    NoPatternInbound {category: String, assign_as_income: bool},
    NoPatternOutbound {category: String, assign_as_expense: bool, assign_as_personal: bool}
}

fn serialise(structure: &Yaml) -> String {
    let mut out = String::new();
    let mut emitter = YamlEmitter::new(&mut out);

    let _ = emitter.dump(structure);
    out
}

fn deserialise(contents: String) -> Result<NewConfig, ScanError> {
    let mut config = YamlLoader::load_from_str(&contents)?;
    let contents = config.pop().unwrap().into_hash().unwrap();
    Ok(NewConfig::new(contents))
}

fn get_date_boundaries(start_date_string: &str) -> Result<(NaiveDate, NaiveDate), ParseError> {
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
        fs::write(&config_path_new, serialise(&NewConfig::template())).expect("Could not write new config file");
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

        let classification = match new_config.find_pattern(&entry) {
            Some(p) => {
                match p {
                    Pattern::Inbound { snippet, category, assign_as_income } => Classification::ExistingPattern(&p, None),
                    Pattern::Outbound  { snippet, category, assign_as_expense, assign_as_personal, require_confirmation } => {
                        let sphere_override = match require_confirmation {
                            true => {
                                match ui.sphere_override(assign_as_personal) {
                                    true => Some(PatternOverride { is_personal: !assign_as_personal }),
                                    false => None,
                                }
                            }
                            false => None
                        };
                        Classification::ExistingPattern(&p, sphere_override)
                    }
                }
            },
            None => {
                let is_personal: Option<bool>;
                ui.display_categories(&new_config.categories);

                let category = ui.capture_category(&new_config.categories);

                let is_transfer = ui.is_transfer();
                let sphere = match is_transfer {
                    true => {
                        Sphere::Personal
                    },
                    false => {
                        match entry.direction {
                            Direction::Outbound => {
                                match ui.is_personal() {
                                    true => Sphere::Personal,
                                    false => Sphere::Work
                                }
                            },
                            Direction::Inbound => {
                                Sphere::Personal
                            }
                        }
                    }
                };

                let create_pattern = ui.create_pattern();

                match create_pattern {
                    false => {
                        match entry.direction {
                            Direction::Outbound => {
                                Classification::NoPatternOutbound {
                                    category: category,
                                    assign_as_expense: !is_transfer,
                                    assign_as_personal: match sphere {
                                        Sphere::Personal => true,
                                        Sphere::Work => false
                                    },
                                }
                            },
                            Direction::Inbound => {
                                Classification::NoPatternInbound {
                                    category: category,
                                    assign_as_income: !is_transfer,
                                }
                            }
                        }
                    },
                    true => {
                        let snippet = ui.snippet();
                        let require_confirmation = match entry.direction {
                            Direction::Inbound => false,
                            Direction::Outbound => {
                                match is_transfer {
                                    true => false,
                                    false => {
                                        let is_personal = match sphere {
                                            Sphere::Personal => true,
                                            Sphere::Work => false
                                        };
                                        ui.require_confirmation(is_personal)
                                    }
                                }
                            }
                        };
                        match entry.direction {
                            Direction::Outbound => {
                                Classification::NewPatternOutbound {
                                    snippet: snippet,
                                    category: category.clone(),
                                    assign_as_expense: !is_transfer,
                                    assign_as_personal: match sphere {
                                        Sphere::Personal => true,
                                        Sphere::Work => false
                                    },
                                    require_confirmation: require_confirmation
                                }
                            },
                            Direction::Inbound => {
                                Classification::NewPatternInbound {
                                    snippet: snippet,
                                    category: category,
                                    assign_as_income: !is_transfer,
                                }
                            }
                        }

                    }
                }
            }

        };

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
