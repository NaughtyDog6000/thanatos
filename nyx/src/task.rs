use std::collections::HashMap;

use crate::item::{ItemKind, RecipeOutput, Tag};

pub type Quantity = u32;

#[derive(Debug, Default, Clone)]
pub struct Query {
    pub tags: Vec<Tag>,
}

impl Query {
    pub fn query(&self, tags: &[Tag]) -> bool {
        self.tags.iter().all(|tag| tags.contains(tag))
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Statistic {
    Gathered(ItemKind),
    Crafted(RecipeOutput),
}

pub struct Statistics(pub HashMap<Statistic, Quantity>);

impl Statistics {
    pub fn with_tags(&self, query: &Query) -> Quantity {
        self.0
            .iter()
            .filter(|(statistic, _)| match statistic {
                Statistic::Gathered(item) | Statistic::Crafted(RecipeOutput::Item(item)) => {
                    query.query(&item.tags())
                }
                Statistic::Crafted(RecipeOutput::Equipment(equipment)) => {
                    query.query(&equipment.tags())
                }
            })
            .map(|(_, quantity)| *quantity)
            .sum()
    }
}

pub enum Reward {
    Proficiency(Query, f32)
}

pub struct Task {
    pub query: Query,
    pub required: Quantity,
    pub rewards: Vec<Reward>
}

impl Task {
    pub fn is_complete(&self, statistics: &Statistics) -> bool {
        statistics.with_tags(&self.query) <= self.required
    }
}

#[derive(Debug, Default)]
pub struct Proficiency(pub Vec<(Query, f32)>);
impl Proficiency {
    pub fn get(&self, tags: &[Tag]) -> f32 {
        self.0
            .iter()
            .filter(|(query, _)| query.query(tags))
            .map(|(_, bonus)| *bonus)
            .sum::<f32>()
    }
}

#[derive(Debug, Default)]
pub struct Proficiencies {
    pub rank_up: Proficiency
}
