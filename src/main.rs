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
use bank_statement_importer::report::ActivityReport;
use bank_statement_importer::ui::UI;

#[test]
fn test_category_equality() {
    assert_eq!(
        Category {name: String::from("foo"), patterns: vec![String::from("bar")]},
        Category {name: String::from("foo"), patterns: vec![String::from("bar")]},
               );

    assert_ne!(
        Category {name: String::from("baz"), patterns: vec![String::from("bar")]},
        Category {name: String::from("foo"), patterns: vec![String::from("bar")]},
               );

    assert_ne!(
        Category {name: String::from("foo"), patterns: vec![String::from("baz")]},
        Category {name: String::from("foo"), patterns: vec![String::from("bar")]},
               );
}

#[derive(Debug)]
struct Category {
    name: String,
    patterns: Vec<String>
}

impl Category {
    fn matches_description(&self, description: &str) -> bool {
        let desc = String::from(description);
        let mut iter = self.patterns.iter().filter(|p| desc.contains(&p[..]) );

        match iter.next() {
            Some(_) => true,
            None => false
        }
    }
}

impl PartialEq for Category {
    fn eq(&self, other: &Category) -> bool {
        self.name == other.name && self.patterns == other.patterns
    }
}

#[test]
fn test_adding_categories_to_catalogue() {
    let mut catalogue = CategoryCatalogue { categories: Vec::new() };

    catalogue.add_category("foo");
    assert_eq!(catalogue.categories, vec![Category { name: String::from("foo"), patterns: Vec::new() }]);

    catalogue.add_category("bar");
    assert_eq!(
        catalogue.categories,
        vec![
            Category { name: String::from("foo"), patterns: Vec::new() },
            Category { name: String::from("bar"), patterns: Vec::new() },
        ]);

    catalogue.add_category("foo");
    assert_eq!(
        catalogue.categories,
        vec![
            Category { name: String::from("foo"), patterns: Vec::new() },
            Category { name: String::from("bar"), patterns: Vec::new() },
        ]);
}

struct CategoryCatalogue {
    categories: Vec<Category>
}

impl CategoryCatalogue {
    fn new(categories: &Vec<Yaml>, patterns: &LinkedHashMap<Yaml, Yaml>) -> CategoryCatalogue {
        let mut catalogue_categories = Vec::new();
        for c in categories {
            let cat = c.as_str().unwrap();
            let mut cat_patterns = Vec::new();
            for p in patterns[c].as_vec().unwrap().iter() {
                cat_patterns.push(String::from(p.as_str().unwrap()))
            }
            let category = Category { name: String::from(cat), patterns: cat_patterns };
            catalogue_categories.push(category);
        }
        CategoryCatalogue {
            categories: catalogue_categories
        }
    }

    fn add_category(&mut self, category: &str) {
        if !self.category_exists(category) {
            self.categories.push(Category { name: String::from(category), patterns: Vec::new() });
        }
    }

    fn category_exists(&self, category: &str) -> bool {
        let mut iter = self.categories.iter().filter(|c| c.name == category);

        match iter.next() {
            Some(_) => true,
            None => false
        }
    }

    fn find_cat(&mut self, category: &str) -> &mut Category{
        let mut iter = self.categories.iter_mut().filter(|x| x.name == category);
        iter.next().unwrap()
    }
}

struct Config {
    personal: CategoryCatalogue,
    work: CategoryCatalogue
}

impl Config {
    fn new(contents: LinkedHashMap<Yaml, Yaml>) -> Config {
        let categories = contents[&Yaml::from_str("categories")].as_hash().unwrap();
        let patterns = contents[&Yaml::from_str("patterns")].as_hash().unwrap();
        Config {
            personal: CategoryCatalogue::new(
                          categories[&Yaml::from_str("personal")].as_vec().unwrap(),
                          patterns[&Yaml::from_str("personal")].as_hash().unwrap(),
                          ),
            work: CategoryCatalogue::new(
                          categories[&Yaml::from_str("work")].as_vec().unwrap(),
                          patterns[&Yaml::from_str("work")].as_hash().unwrap(),
                          ),
        }
    }

    fn personal_categories(&self) -> Vec<String> {
        self.personal.categories.iter().map(|x| x.name.clone()).collect()
    }

    fn work_categories(&self) -> Vec<String> {
        self.work.categories.iter().map(|x| x.name.clone()).collect()
    }

    fn find_cat(&mut self, entry_type: &str, category: &str) -> &mut Category {
        if entry_type == "personal" {
            self.personal.find_cat(category)
        } else {
            self.work.find_cat(category)
        }
    }

    fn match_category(&self, entry_type: &str, entry: &str) -> Option<&Category> {
        let cats = if entry_type == "personal" {
            &self.personal.categories
        } else {
            &self.work.categories
        };

        let mut iter = cats.iter().filter(|c| c.matches_description(entry));
        iter.next()
    }

    fn add_category(&mut self, entry_type: &str, category: &str) {
        if entry_type == "personal" {
            &self.personal.add_category(category);
        } else {
            &self.work.add_category(category);
        }
    }

    fn template() -> Yaml {
        let mut config: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
        let patterns: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
        let mut types: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
        types.insert(Yaml::from_str("personal"), Yaml::Array(Vec::new()));
        types.insert(Yaml::from_str("work"), Yaml::Array(Vec::new()));
        config.insert(Yaml::from_str("categories"), Yaml::Hash(types));
        config.insert(Yaml::from_str("patterns"), Yaml::Hash(patterns));
        Yaml::Hash(config)
    }

    fn export(&self) -> Yaml {
        let mut config: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
        let mut new_categories: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
        let mut personal_categories = Vec::new();
        let mut work_categories = Vec::new();
        let mut patterns: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
        let mut personal_patterns: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
        let mut work_patterns: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
        for cat in self.personal.categories.iter()  {
            personal_categories.push(Yaml::from_str(&cat.name));
            let patterns: Vec<Yaml> = cat.patterns.iter().map(|x| Yaml::from_str(x)).collect();
            personal_patterns.insert(Yaml::from_str(&cat.name), Yaml::Array(patterns));
        }
        for cat in self.work.categories.iter()  {
            work_categories.push(Yaml::from_str(&cat.name));
            let patterns: Vec<Yaml> = cat.patterns.iter().map(|x| Yaml::from_str(x)).collect();
            work_patterns.insert(Yaml::from_str(&cat.name), Yaml::Array(patterns));
        }
        new_categories.insert(Yaml::from_str("personal"), Yaml::Array(personal_categories));
        new_categories.insert(Yaml::from_str("work"), Yaml::Array(work_categories));
        patterns.insert(Yaml::from_str("personal"), Yaml::Hash(personal_patterns));
        patterns.insert(Yaml::from_str("work"), Yaml::Hash(work_patterns));
        config.insert(Yaml::from_str("categories"), Yaml::Hash(new_categories));
        config.insert(Yaml::from_str("patterns"), Yaml::Hash(patterns));
        Yaml::Hash(config)
    }
}


fn config_template() -> Yaml {
    let mut config: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
    let mut patterns: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
    let mut types: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
    types.insert(Yaml::from_str("personal"), Yaml::Array(Vec::new()));
    types.insert(Yaml::from_str("work"), Yaml::Array(Vec::new()));
    patterns.insert(Yaml::from_str("personal"), Yaml::Hash(LinkedHashMap::new()));
    patterns.insert(Yaml::from_str("work"), Yaml::Hash(LinkedHashMap::new()));
    config.insert(Yaml::from_str("categories"), Yaml::Hash(types));
    config.insert(Yaml::from_str("patterns"), Yaml::Hash(patterns));
    Yaml::Hash(config)
}

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
    let mut config_path = dirs::home_dir().expect("Can't find home dir");
    config_path.push(".bank_statement_importer.yml");

    if config_path.exists() {
        println!("Config exists at {:?}", config_path);
    } else {
        fs::write(&config_path, serialise(&config_template())).expect("Could not write config file");
        println!("Config created at {:?}", config_path);
    }

    let mut config = deserialise(fs::read_to_string(&config_path).expect("Could not read config file")).expect("Could not parse config contents");

    let args : Vec<String>= env::args().collect();

    let input_directory_path = &args[1];
    let start_date_string = &args[2];

    let (start_date, end_date_excl) = get_date_boundaries(start_date_string).expect("Could not set date boundaries");

    let mut raw_entries: Vec<StringRecord> = Vec::new();

    for file in fs::read_dir(input_directory_path).unwrap() {
        let mut reader = Reader::from_path(file.unwrap().path()).unwrap();
        for result in reader.records() {
            let record = result.unwrap();
            let record_date = NaiveDate::parse_from_str(record.get(0).unwrap(), "%Y%m%d").unwrap();
            if record_date >= start_date && record_date < end_date_excl {
                raw_entries.push(record);
            }
        }
    }

    let mut classification = HashMap::new();
    let personal_entries: HashMap<String, Vec<f32>> = HashMap::new();
    let work_entries: HashMap<String, Vec<f32>> = HashMap::new();
    classification.insert(String::from("personal"), personal_entries);
    classification.insert(String::from("work"), work_entries);

    let mut report = ActivityReport::new();
    let ui = UI {};

    for entry in raw_entries {
        
        ui.display_entry(&entry);

        let entry_type = ui.get_type();

        let selected_category = match config.match_category(entry_type, entry.get(1).unwrap()) {
            Some(c) => {
                ui.display_automap(&c.name);
                c.name.clone()
            },
            None => {
                    if entry_type == "personal" {
                        ui.display_categories("personal", &config.personal_categories());
                    } else {
                        ui.display_categories("work", &config.work_categories());
                    }

                    let category = ui.capture_category();

                    config.add_category(entry_type, category.trim());

                    if let Some(pattern) = ui.capture_pattern() {
                        let category = config.find_cat(entry_type, category.trim());
                        category.patterns.push(String::from(pattern));
                    }

                    fs::write(&config_path, serialise(&config.export())).expect("Could not write config file");

                    config = deserialise(fs::read_to_string(&config_path).expect("Could not read config file")).expect("Could not parse config contents");

                    category
            }
        };

        if entry_type == "personal" && config.personal_categories().contains(&String::from(selected_category.trim())) {
            report.add_entry(entry_type, selected_category.trim(), &entry);
        } else if entry_type == "work" && config.work_categories().contains(&String::from(selected_category.trim())) {
            report.add_entry(entry_type, selected_category.trim(), &entry);
        } else {
            println!("Category {} does not exist", selected_category);
            exit(3);
        }
    }

    println!("Work Expense: {}", report.total("work", true));
    println!("Personal Expense: {}", report.total("personal", true));
}
