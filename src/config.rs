use crate::entry_manager::Pattern;
use crate::raw_entry::{RawEntry, Direction};
use linked_hash_map::LinkedHashMap;
use yaml_rust::Yaml;

#[cfg(test)]
mod tests {
    use csv::StringRecord;
    use linked_hash_map::LinkedHashMap;
    use yaml_rust::Yaml;

    use super::*;
    #[test]
    fn test_creating_config_template() {
        let mut expected: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
        expected.insert(Yaml::from_str("categories"), Yaml::Array(Vec::new()));
        expected.insert(Yaml::from_str("inbound_patterns"), Yaml::Array(Vec::new()));
        expected.insert(Yaml::from_str("outbound_patterns"), Yaml::Array(Vec::new()));

        assert_eq!(Yaml::Hash(expected), Config::template());
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

        let config = Config::new(config_input);

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

        let config = Config {
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
}

#[derive(Debug)]
pub struct Config {
    pub categories: Vec<String>,
    pub inbound_patterns: Vec<Pattern>,
    pub outbound_patterns: Vec<Pattern>
}

impl Config {
    pub fn new(contents: LinkedHashMap<Yaml, Yaml>) -> Config {
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
        Config {
            categories: categories,
            inbound_patterns: inbound_patterns,
            outbound_patterns: outbound_patterns
        }
    }

    pub fn find_pattern(&self, entry: &RawEntry) -> Option<&Pattern> {
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

    pub fn export(&self) -> Yaml {
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

    pub fn template() -> Yaml {
        let mut config: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
        config.insert(Yaml::from_str("categories"), Yaml::Array(Vec::new()));
        config.insert(Yaml::from_str("inbound_patterns"), Yaml::Array(Vec::new()));
        config.insert(Yaml::from_str("outbound_patterns"), Yaml::Array(Vec::new()));
        Yaml::Hash(config)
    }
}
