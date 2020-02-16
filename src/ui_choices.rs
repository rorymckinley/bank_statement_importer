use crate::Sphere;
use crate::entry_manager::Pattern;

pub struct UiChoices<'a> {
    pub existing_pattern: Option<&'a Pattern>,
    pub pattern_override: Option<PatternOverride>,
    pub category: Option<String>,
    pub transfer: Option<bool>,
    pub sphere: Option<Sphere>,
    pub create_pattern: Option<bool>,
    pub snippet: Option<String>,
    pub require_confirmation: Option<bool>
}

#[derive(Debug, PartialEq)]
pub struct PatternOverride {
    pub is_personal: bool
}
