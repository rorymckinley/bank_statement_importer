use std::{env, fs};
use std::process::Command;
use csv::Writer;
use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use rexpect::session::spawn_command;
use rexpect::errors::*;
use serde::{Deserialize, Serialize};

#[test]
fn test_happy_day_empty_config() {
    let home = assert_fs::TempDir::new().unwrap();
    let config_file = home.child(".bank_statement_importer.yml.new");
    let storage = assert_fs::TempDir::new().unwrap();

    setup_fixtures(&storage);
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.args(&[storage.path().to_str().unwrap(), "20191201"])
       .env("HOME", home.path().to_str().unwrap());

    let mut p = spawn_command(cmd, Some(500)).expect("Spawning command");

    let mut instructions = first_batch_of_instructions();
    instructions.extend(second_batch_of_instructions());

    for instruction in instructions {
        println!("{}", instruction.entry);
        p.exp_regex(&instruction.entry).expect("Entry");
        if instruction.duplicate {
            p.exp_regex("An entry identical to this one has already been processed during this run\r\nWould you like to skip this entry \\[y/n\\]\\?").expect("Prompt about duplicate record");
            if instruction.discard {
                p.send_line("y").expect("Skip this entry");
                continue
            } else {
                p.send_line("n").expect("Do not skip this entry");
            }
        }
        if !instruction.matches_existing_pattern {
            p.exp_regex("Enter the existing category, or leave blank").expect("Prompt");
            if instruction.new_category {
                p.send_line("").expect("select new cat");
                p.exp_regex("New category:").expect("New cat prompt");
                p.send_line(&instruction.category).expect("new cat");
            } else {
                p.send_line(&instruction.category).expect("Existing cat");
            }

            p.exp_regex("Does this entry represent a transfer between accounts\\? \\[n/y\\]").expect("Transfer prompt");
            if instruction.transfer {
                p.send_line("y").expect("A transfer response");
            } else {
                p.send_line("n").expect("A non-transfer response");
            }

            match instruction.direction {
                Direction::Outbound => {
                    if !instruction.transfer {
                        p.exp_regex("Is this a work or a personal entry \\[p/w\\]\\?").expect("Personal prompt");
                        if instruction.personal {
                            p.send_line("p").expect("Personal");
                        } else {
                            p.send_line("w").expect("Work");
                        }
                    }
                },
                Direction::Inbound => ()
            }

            p.exp_regex("Would you like to create a pattern from this entry \\[y/n\\]\\?").expect("Create pattern prompt");
            if instruction.create_pattern {
                p.send_line("y").expect("Pattern");
                p.exp_regex("Please provide the snippet").expect("Snippet prompt");
                if let Some(snippet) = instruction.snippet {
                    p.send_line(&snippet).expect("Snippet");
                }

                match instruction.direction {
                    Direction::Outbound => {
                        if !instruction.transfer {
                            let assigned_to = if instruction.personal {
                                "personal"
                            } else {
                                "work"
                            };
                            p.exp_regex(&format!("Should this always be assigned to {} \\[y/n\\]\\?", assigned_to))
                             .expect("Confirmation prompt");
                            if instruction.require_confirmation {
                                p.send_line("n").expect("Require confirmation");
                            } else {
                                p.send_line("y").expect("Do not require confirmation");
                            }
                        }
                    },
                    Direction::Inbound => ()
                }
            } else {
                p.send_line("n").expect("NO Pattern");
            }
        } else {
            if instruction.require_confirmation {
                if instruction.existing_pattern_is_personal {
                    p.exp_regex("This type of entry is normally assigned to personal, would you prefer to assing this entry to work [y/n]?").expect("Assign to work pattern override prompt");
                } else {
                    p.exp_regex("This type of entry is normally assigned to work, would you prefer to assing this entry to personal [y/n]?").expect("Assign to personal pattern override prompt");
                }

                if instruction.override_pattern_assignment {
                    p.send_line("y").expect("Override pattern");
                } else {
                    p.send_line("n").expect("Do not override pattern");
                }
            }
        }
    }

    p.exp_regex("Personal Expense: 2129.75").expect("Personal Total");
    p.exp_regex("Work Expense: 1809.32").expect("Work total");
    p.exp_regex("Categorised Personal Expenses").expect("Personal Expenses Heading");

    p.exp_regex("books 1222.21").expect("Books total");
    p.exp_regex("2019-12-01 Fake Bookshop 333.33").expect("First personal book entry");
    p.exp_regex("2019-12-01 Fake Bookshop 333.33").expect("Second personal book entry");
    p.exp_regex("2019-12-12 Fake Bookshop 555.55").expect("Second personal book entry");

    p.exp_regex("salary 0.00").expect("Salary total");

    p.exp_regex("groceries 312.32").expect("Groceries total");
    p.exp_regex("2019-12-03 Fake Supermarket 123.45").expect("First personal grocery entry");
    p.exp_regex("2019-12-09 Ad Hoc 3 99.99").expect("Second personal grocery entry");
    p.exp_regex("2019-12-13 Fake Supermarket 88.88").expect("Third personal grocery entry");

    p.exp_regex("transfer 0.00").expect("Transfer total");

    p.exp_regex("saas 595.22").expect("Personal SAAS total");
    p.exp_regex("2019-12-17 SAAS as a service 595.22").expect("First Personal SAAS entry");

    p.exp_regex("lottery 0.00").expect("Lottery total");

    p.exp_regex("Categorised Work Expenses").expect("Work Expense Heading");

    p.exp_regex("books 444.44").expect("Work books total");
    p.exp_regex("2019-12-11 Fake Bookshop 444.44").expect("First work book entry");

    p.exp_regex("saas 1364.88").expect("Work SAAS total");
    p.exp_regex("2019-12-04 SAAS as a service 253.77").expect("Work SAAS first entry");
    p.exp_regex("2019-12-10 Ad Hoc 4 1111.11").expect("Work SAAS second entry");

    let categories = vec![
        "books".into(),
        "salary".into(),
        "groceries".into(),
        "transfer".into(),
        "saas".into(),
        "lottery".into(),
    ];

    let inbound_patterns = vec![
        ExpectedInboundPattern {
            snippet: "Salary".into(),
            category: "salary".into(),
            assign_as_income: true
        },
        ExpectedInboundPattern {
            snippet: "Transfer In".into(),
            category: "transfer".into(),
            assign_as_income: false
        },
    ];

    let outbound_patterns = vec![
        ExpectedOutboundPattern {
            snippet: "Fake Bookshop".into(),
            assign_as_expense: true,
            assign_as_personal: true,
            category: "books".into(),
            require_confirmation: true
        },
        ExpectedOutboundPattern {
            snippet: "Fake Supermarket".into(),
            assign_as_expense: true,
            assign_as_personal: true,
            category: "groceries".into(),
            require_confirmation: false
        },
        ExpectedOutboundPattern {
            snippet: "SAAS".into(),
            assign_as_expense: true,
            assign_as_personal: false,
            category: "saas".into(),
            require_confirmation: true
        },
        ExpectedOutboundPattern {
            snippet: "Transfer Out".into(),
            assign_as_expense: false,
            assign_as_personal: true,
            category: "transfer".into(),
            require_confirmation: false
        },
    ];

    let expected_config = ExpectedConfig {
        categories,
        inbound_patterns,
        outbound_patterns,
    };

    let created_config: ExpectedConfig = serde_yaml::from_str(
        &fs::read_to_string(config_file.path()).expect("Read config into file")
        ).expect("Deserialising config");

    assert_eq!(expected_config, created_config);
}

#[test]
fn test_happy_day_populated_config() {
    let home = assert_fs::TempDir::new().unwrap();
    let config_file = home.child(".bank_statement_importer.yml.new");
    let storage = assert_fs::TempDir::new().unwrap();

    setup_fixtures(&storage);
    setup_config(&config_file);
  
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.args(&[storage.path().to_str().unwrap(), "20191201"])
       .env("HOME", home.path().to_str().unwrap());

    let mut p = spawn_command(cmd, Some(500)).expect("Spawning command");

    let mut instructions = first_batch_of_instructions_populated_config();
    instructions.extend(second_batch_of_instructions_populated_config());

    for instruction in instructions {
        println!("{}", instruction.entry);
        p.exp_regex(&instruction.entry).expect("Entry");
        if instruction.duplicate {
            p.exp_regex("An entry identical to this one has already been processed during this run\r\nWould you like to skip this entry \\[y/n\\]\\?").expect("Prompt about duplicate record");
            if instruction.discard {
                p.send_line("y").expect("Skip this entry");
                continue
            } else {
                p.send_line("n").expect("Do not skip this entry");
            }
        }
        if !instruction.matches_existing_pattern {
            p.exp_regex("Enter the existing category, or leave blank").expect("Prompt");
            if instruction.new_category {
                p.send_line("").expect("select new cat");
                p.exp_regex("New category:").expect("New cat prompt");
                p.send_line(&instruction.category).expect("new cat");
            } else {
                p.send_line(&instruction.category).expect("Existing cat");
            }

            p.exp_regex("Does this entry represent a transfer between accounts\\? \\[n/y\\]").expect("Transfer prompt");
            if instruction.transfer {
                p.send_line("y").expect("A transfer response");
            } else {
                p.send_line("n").expect("A non-transfer response");
            }

            match instruction.direction {
                Direction::Outbound => {
                    if !instruction.transfer {
                        p.exp_regex("Is this a work or a personal entry \\[p/w\\]\\?").expect("Personal prompt");
                        if instruction.personal {
                            p.send_line("p").expect("Personal");
                        } else {
                            p.send_line("w").expect("Work");
                        }
                    }
                },
                Direction::Inbound => ()
            }

            p.exp_regex("Would you like to create a pattern from this entry \\[y/n\\]\\?").expect("Create pattern prompt");
            if instruction.create_pattern {
                p.send_line("y").expect("Pattern");
                p.exp_regex("Please provide the snippet").expect("Snippet prompt");
                if let Some(snippet) = instruction.snippet {
                    p.send_line(&snippet).expect("Snippet");
                }

                match instruction.direction {
                    Direction::Outbound => {
                        if !instruction.transfer {
                            let assigned_to = if instruction.personal {
                                "personal"
                            } else {
                                "work"
                            };
                            p.exp_regex(&format!("Should this always be assigned to {} \\[y/n\\]\\?", assigned_to))
                             .expect("Confirmation prompt");
                            if instruction.require_confirmation {
                                p.send_line("n").expect("Require confirmation");
                            } else {
                                p.send_line("y").expect("Do not require confirmation");
                            }
                        }
                    },
                    Direction::Inbound => ()
                }
            } else {
                p.send_line("n").expect("NO Pattern");
            }
        } else {
            if instruction.require_confirmation {
                if instruction.existing_pattern_is_personal {
                    p.exp_regex("This type of entry is normally assigned to personal, would you prefer to assing this entry to work [y/n]?").expect("Assign to work pattern override prompt");
                } else {
                    p.exp_regex("This type of entry is normally assigned to work, would you prefer to assing this entry to personal [y/n]?").expect("Assign to personal pattern override prompt");
                }

                if instruction.override_pattern_assignment {
                    p.send_line("y").expect("Override pattern");
                } else {
                    p.send_line("n").expect("Do not override pattern");
                }
            }
        }
    }

    p.exp_regex("Personal Expense: 2129.75").expect("Personal Total");
    p.exp_regex("Work Expense: 1809.32").expect("Work total");
    p.exp_regex("Categorised Personal Expenses").expect("Personal Expenses Heading");

    p.exp_regex("books 1222.21").expect("Books total");
    p.exp_regex("2019-12-01 Fake Bookshop 333.33").expect("First personal book entry");
    p.exp_regex("2019-12-01 Fake Bookshop 333.33").expect("Second personal book entry");
    p.exp_regex("2019-12-12 Fake Bookshop 555.55").expect("Second personal book entry");

    p.exp_regex("transfer 0.00").expect("Transfer total");

    p.exp_regex("lottery 0.00").expect("Lottery total");

    p.exp_regex("salary 0.00").expect("Salary total");


    p.exp_regex("groceries 312.32").expect("Groceries total");
    p.exp_regex("2019-12-03 Fake Supermarket 123.45").expect("First personal grocery entry");
    p.exp_regex("2019-12-09 Ad Hoc 3 99.99").expect("Second personal grocery entry");
    p.exp_regex("2019-12-13 Fake Supermarket 88.88").expect("Third personal grocery entry");

    p.exp_regex("saas 595.22").expect("Personal SAAS total");
    p.exp_regex("2019-12-17 SAAS as a service 595.22").expect("First Personal SAAS entry");


    p.exp_regex("Categorised Work Expenses").expect("Work Expense Heading");

    p.exp_regex("books 444.44").expect("Work books total");
    p.exp_regex("2019-12-11 Fake Bookshop 444.44").expect("First work book entry");

    p.exp_regex("saas 1364.88").expect("Work SAAS total");
    p.exp_regex("2019-12-04 SAAS as a service 253.77").expect("Work SAAS first entry");
    p.exp_regex("2019-12-10 Ad Hoc 4 1111.11").expect("Work SAAS second entry");

    let categories = vec![
        "books".into(),
        "transfer".into(),
        "lottery".into(),
        "salary".into(),
        "groceries".into(),
        "saas".into(),
    ];

    let inbound_patterns = vec![
        ExpectedInboundPattern {
            snippet: "Transfer In".into(),
            category: "transfer".into(),
            assign_as_income: false
        },
        ExpectedInboundPattern {
            snippet: "Salary".into(),
            category: "salary".into(),
            assign_as_income: true
        },
    ];

    let outbound_patterns = vec![
        ExpectedOutboundPattern {
            snippet: "Fake Bookshop".into(),
            assign_as_expense: true,
            assign_as_personal: true,
            category: "books".into(),
            require_confirmation: true
        },
        ExpectedOutboundPattern {
            snippet: "Fake Supermarket".into(),
            assign_as_expense: true,
            assign_as_personal: true,
            category: "groceries".into(),
            require_confirmation: false
        },
        ExpectedOutboundPattern {
            snippet: "SAAS".into(),
            assign_as_expense: true,
            assign_as_personal: false,
            category: "saas".into(),
            require_confirmation: true
        },
        ExpectedOutboundPattern {
            snippet: "Transfer Out".into(),
            assign_as_expense: false,
            assign_as_personal: true,
            category: "transfer".into(),
            require_confirmation: false
        },
    ];

    let expected_config = ExpectedConfig {
        categories,
        inbound_patterns,
        outbound_patterns,
    };

    let created_config: ExpectedConfig = serde_yaml::from_str(
        &fs::read_to_string(config_file.path()).expect("Read config into file")
        ).expect("Deserialising config");

    assert_eq!(expected_config, created_config);
}

fn setup_fixtures(storage: &assert_fs::fixture::TempDir) {
    let dump_2 = storage.child("b.csv");
    let mut wtr_b = Writer::from_path(dump_2.path()).expect("Writer for file B");

    wtr_b.write_record(&["Date", "Description", "Amount", "Balance"]).expect("Writing record");
    wtr_b.write_record(&["20191130", "Insane purchase outside our date range", "-8888.88", "0.00"]).expect("Writing record");
    wtr_b.write_record(&["20191201", "Fake Bookshop", "-333.33", "0.00"]).expect("Writing record");
    wtr_b.write_record(&["20191203", "Fake Supermarket", "-123.45", "0.00"]).expect("Writing record");
    wtr_b.write_record(&["20191211", "Fake Bookshop", "-444.44", "0.00"]).expect("Writing record");
    wtr_b.write_record(&["20191212", "Fake Bookshop", "-555.55", "0.00"]).expect("Writing record");
    wtr_b.write_record(&["20191213", "Fake Supermarket", "-88.88", "0.00"]).expect("Writing record");
    wtr_b.write_record(&["20191214", "Salary", "222.22", "0.00"]).expect("Writing record");
    wtr_b.write_record(&["20191215", "Transfer In", "90.91", "0.00"]).expect("Writing record");
    wtr_b.write_record(&["20191216", "Transfer Out", "-60.61", "0.00"]).expect("Writing record");
    wtr_b.write_record(&["20191217", "SAAS as a service", "-595.22", "0.00"]).expect("Writing record");
    wtr_b.write_record(&["20200101", "Insane purchase outside our date range", "-7777.77", "0.00"]).expect("Writing record");
    wtr_b.flush().expect("Flushing");

    let dump_1 = storage.child("a.csv");
    let mut wtr_a = Writer::from_path(dump_1.path()).expect("Writer for file A");

    wtr_a.write_record(&["Date", "Description", "Amount", "Balance"]).expect("Writing record");
    wtr_a.write_record(&["20191130", "Insane purchase outside our date range", "-9999.99", "0.00"]).expect("Writing record");
    wtr_a.write_record(&["20191201", "Fake Bookshop", "-333.33", "0.00"]).expect("Writing record");
    wtr_a.write_record(&["20191202", "Salary", "555.55", "0.00"]).expect("Writing record");
    wtr_a.write_record(&["20191203", "Fake Supermarket", "-123.45", "0.00"]).expect("Writing record");
    wtr_a.write_record(&["20191204", "Transfer In", "50.51", "0.00"]).expect("Writing record");
    wtr_a.write_record(&["20191204", "SAAS as a service", "-253.77", "0.00"]).expect("Writing record");
    wtr_a.write_record(&["20191205", "Transfer Out", "-70.71", "0.00"]).expect("Writing record");
    wtr_a.write_record(&["20191206", "Ad Hoc 1", "-20.21", "0.00"]).expect("Writing record");
    wtr_a.write_record(&["20191207", "Ad Hoc 2", "10.11", "0.00"]).expect("Writing record");
    wtr_a.write_record(&["20191208", "Lotto", "777.77", "0.00"]).expect("Writing record");
    wtr_a.write_record(&["20191209", "Ad Hoc 3", "-99.99", "0.00"]).expect("Writing record");
    wtr_a.write_record(&["20191210", "Ad Hoc 4", "-1111.11", "0.00"]).expect("Writing record");
    wtr_a.write_record(&["20200101", "Insane purchase outside our date range", "-9999.98", "0.00"]).expect("Writing record");
    wtr_a.flush().expect("Flushing");

}

fn setup_config(config_file: &assert_fs::fixture::ChildPath) {
    let categories = vec![
        "books".into(),
        "transfer".into(),
        "lottery".into(),
    ];

    let inbound_patterns = vec![
        ExpectedInboundPattern {
            snippet: "Transfer In".into(),
            category: "transfer".into(),
            assign_as_income: false
        },
    ];

    let outbound_patterns = vec![
        ExpectedOutboundPattern {
            snippet: "Fake Bookshop".into(),
            assign_as_expense: true,
            assign_as_personal: true,
            category: "books".into(),
            require_confirmation: true
        },
    ];

    let existing_config = ExpectedConfig {
        categories,
        inbound_patterns,
        outbound_patterns,
    };

    fs::write(config_file.path(), serde_yaml::to_string(&existing_config).expect("Serialising config"));

}

#[derive(Debug)]
struct Instruction {
    entry: String,
    direction: Direction,
    new_category: bool,
    category: String,
    transfer: bool,
    personal: bool,
    create_pattern: bool,
    snippet: Option<String>,
    require_confirmation: bool,
    matches_existing_pattern: bool,
    existing_pattern_is_personal: bool,
    override_pattern_assignment: bool,
    duplicate: bool,
    discard: bool,
}

#[derive(Debug)]
enum Direction {
    Inbound, 
    Outbound
}

fn first_batch_of_instructions() -> Vec<Instruction> {
    vec![
        Instruction {
            entry: "2019-12-01 outbound Fake Bookshop 333.33 0.00".into(),
            direction: Direction::Outbound,
            new_category: true,
            category: "books".into(),
            transfer: false,
            personal: true,
            create_pattern: true,
            snippet: Some("Fake Bookshop".into()),
            require_confirmation: true,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-02 inbound Salary 555.55 0.00".into(),
            direction: Direction::Inbound,
            new_category: true,
            category: "salary".into(),
            transfer: false,
            personal: true,
            create_pattern: true,
            snippet: Some("Salary".into()),
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-03 outbound Fake Supermarket 123.45 0.00".into(),
            direction: Direction::Outbound,
            new_category: true,
            category: "groceries".into(),
            transfer: false,
            personal: true,
            create_pattern: true,
            snippet: Some("Fake Supermarket".into()),
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-04 inbound Transfer In 50.51 0.00".into(),
            direction: Direction::Inbound,
            new_category: true,
            category: "transfer".into(),
            transfer: true,
            personal: true,
            create_pattern: true,
            snippet: Some("Transfer In".into()),
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-04 outbound SAAS as a service 253.77 0.00".into(),
            direction: Direction::Outbound,
            new_category: true,
            category: "saas".into(),
            transfer: false,
            personal: false,
            create_pattern: true,
            snippet: Some("SAAS".into()),
            require_confirmation: true,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-05 outbound Transfer Out 70.71 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "transfer".into(),
            transfer: true,
            personal: true,
            create_pattern: true,
            snippet: Some("Transfer Out".into()),
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-06 outbound Ad Hoc 1 20.21 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "transfer".into(),
            transfer: true,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-07 inbound Ad Hoc 2 10.11 0.00".into(),
            direction: Direction::Inbound,
            new_category: false,
            category: "transfer".into(),
            transfer: true,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-08 inbound Lotto 777.77 0.00".into(),
            direction: Direction::Inbound,
            new_category: true,
            category: "lottery".into(),
            transfer: false,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-09 outbound Ad Hoc 3 99.99 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "groceries".into(),
            transfer: false,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-10 outbound Ad Hoc 4 1111.11 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "saas".into(),
            transfer: false,
            personal: false,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
    ]
}

fn second_batch_of_instructions() -> Vec<Instruction> {
    vec![
        Instruction {
            entry: "2019-12-01 outbound Fake Bookshop 333.33 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "books".into(),
            transfer: false,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: true,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: false,
            duplicate: true,
            discard: false
        },
        Instruction {
            entry: "2019-12-03 outbound Fake Supermarket 123.45 0.00".into(),
            direction: Direction::Outbound,
            new_category: true,
            category: "groceries".into(),
            transfer: false,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: true,
            discard: true
        },
        Instruction {
            entry: "2019-12-11 outbound Fake Bookshop 444.44 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "books".into(),
            transfer: false,
            personal: false,
            create_pattern: false,
            snippet: None,
            require_confirmation: true,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: true,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-12 outbound Fake Bookshop 555.55 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "books".into(),
            transfer: false,
            personal: false,
            create_pattern: false,
            snippet: None,
            require_confirmation: true,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-13 outbound Fake Supermarket 88.88 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "groceries".into(),
            transfer: false,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-14 inbound Salary 222.22 0.00".into(),
            direction: Direction::Inbound,
            new_category: false,
            category: "salary".into(),
            transfer: false,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: true,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-15 inbound Transfer In 90.91 0.00".into(),
            direction: Direction::Inbound,
            new_category: false,
            category: "transfer".into(),
            transfer: true,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-16 outbound Transfer Out 60.61 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "transfer".into(),
            transfer: true,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-17 outbound SAAS as a service 595.22 0.00".into(),
            direction: Direction::Outbound,
            new_category: true,
            category: "saas".into(),
            transfer: false,
            personal: false,
            create_pattern: false,
            snippet: None,
            require_confirmation: true,
            matches_existing_pattern: true,
            existing_pattern_is_personal: false,
            override_pattern_assignment: true,
            duplicate: false,
            discard: false
        },
    ]
}

fn first_batch_of_instructions_populated_config() -> Vec<Instruction> {
    vec![
        Instruction {
            entry: "2019-12-01 outbound Fake Bookshop 333.33 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "books".into(),
            transfer: false,
            personal: true,
            create_pattern: true,
            snippet: Some("Fake Bookshop".into()),
            require_confirmation: true,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-02 inbound Salary 555.55 0.00".into(),
            direction: Direction::Inbound,
            new_category: true,
            category: "salary".into(),
            transfer: false,
            personal: true,
            create_pattern: true,
            snippet: Some("Salary".into()),
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-03 outbound Fake Supermarket 123.45 0.00".into(),
            direction: Direction::Outbound,
            new_category: true,
            category: "groceries".into(),
            transfer: false,
            personal: true,
            create_pattern: true,
            snippet: Some("Fake Supermarket".into()),
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-04 inbound Transfer In 50.51 0.00".into(),
            direction: Direction::Inbound,
            new_category: false,
            category: "transfer".into(),
            transfer: true,
            personal: true,
            create_pattern: false,
            snippet: Some("Transfer In".into()),
            require_confirmation: false,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-04 outbound SAAS as a service 253.77 0.00".into(),
            direction: Direction::Outbound,
            new_category: true,
            category: "saas".into(),
            transfer: false,
            personal: false,
            create_pattern: true,
            snippet: Some("SAAS".into()),
            require_confirmation: true,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-05 outbound Transfer Out 70.71 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "transfer".into(),
            transfer: true,
            personal: true,
            create_pattern: true,
            snippet: Some("Transfer Out".into()),
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-06 outbound Ad Hoc 1 20.21 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "transfer".into(),
            transfer: true,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-07 inbound Ad Hoc 2 10.11 0.00".into(),
            direction: Direction::Inbound,
            new_category: false,
            category: "transfer".into(),
            transfer: true,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-08 inbound Lotto 777.77 0.00".into(),
            direction: Direction::Inbound,
            new_category: false,
            category: "lottery".into(),
            transfer: false,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-09 outbound Ad Hoc 3 99.99 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "groceries".into(),
            transfer: false,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-10 outbound Ad Hoc 4 1111.11 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "saas".into(),
            transfer: false,
            personal: false,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
    ]
}

fn second_batch_of_instructions_populated_config() -> Vec<Instruction> {
    vec![
        Instruction {
            entry: "2019-12-01 outbound Fake Bookshop 333.33 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "books".into(),
            transfer: false,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: true,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: false,
            duplicate: true,
            discard: false
        },
        Instruction {
            entry: "2019-12-03 outbound Fake Supermarket 123.45 0.00".into(),
            direction: Direction::Outbound,
            new_category: true,
            category: "groceries".into(),
            transfer: false,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: false,
            existing_pattern_is_personal: false,
            override_pattern_assignment: false,
            duplicate: true,
            discard: true
        },
        Instruction {
            entry: "2019-12-11 outbound Fake Bookshop 444.44 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "books".into(),
            transfer: false,
            personal: false,
            create_pattern: false,
            snippet: None,
            require_confirmation: true,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: true,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-12 outbound Fake Bookshop 555.55 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "books".into(),
            transfer: false,
            personal: false,
            create_pattern: false,
            snippet: None,
            require_confirmation: true,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-13 outbound Fake Supermarket 88.88 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "groceries".into(),
            transfer: false,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-14 inbound Salary 222.22 0.00".into(),
            direction: Direction::Inbound,
            new_category: false,
            category: "salary".into(),
            transfer: false,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: true,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-15 inbound Transfer In 90.91 0.00".into(),
            direction: Direction::Inbound,
            new_category: false,
            category: "transfer".into(),
            transfer: true,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-16 outbound Transfer Out 60.61 0.00".into(),
            direction: Direction::Outbound,
            new_category: false,
            category: "transfer".into(),
            transfer: true,
            personal: true,
            create_pattern: false,
            snippet: None,
            require_confirmation: false,
            matches_existing_pattern: true,
            existing_pattern_is_personal: true,
            override_pattern_assignment: false,
            duplicate: false,
            discard: false
        },
        Instruction {
            entry: "2019-12-17 outbound SAAS as a service 595.22 0.00".into(),
            direction: Direction::Outbound,
            new_category: true,
            category: "saas".into(),
            transfer: false,
            personal: false,
            create_pattern: false,
            snippet: None,
            require_confirmation: true,
            matches_existing_pattern: true,
            existing_pattern_is_personal: false,
            override_pattern_assignment: true,
            duplicate: false,
            discard: false
        },
    ]
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ExpectedConfig {
    categories: Vec<String>,
    inbound_patterns: Vec<ExpectedInboundPattern>,
    outbound_patterns: Vec<ExpectedOutboundPattern>
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ExpectedInboundPattern {
    snippet: String,
    category: String,
    assign_as_income: bool,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ExpectedOutboundPattern {
    snippet: String,
    category: String,
    assign_as_expense: bool,
    assign_as_personal: bool,
    require_confirmation: bool
}
