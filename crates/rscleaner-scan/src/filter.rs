use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

/// Name, age, and size filters of one path target.
#[derive(Clone, Debug, Default)]
pub struct Filters {
    include: Option<GlobSet>,
    exclude: Option<GlobSet>,
    include_masks: Vec<String>,
    min_age_days: Option<u32>,
    min_size: Option<u64>,
    now_unix: i64,
}

impl Filters {
    pub fn build(
        include: &[String],
        exclude: &[String],
        min_age_days: Option<u32>,
        min_size: Option<u64>,
        now_unix: i64,
    ) -> Result<Self, globset::Error> {
        Ok(Self {
            include: compile(include)?,
            exclude: compile(exclude)?,
            include_masks: include.to_vec(),
            min_age_days,
            min_size,
            now_unix,
        })
    }

    pub fn include_names(&self) -> Vec<String> {
        self.include_masks.clone()
    }

    pub fn min_age_days(&self) -> Option<u32> {
        self.min_age_days
    }

    /// True when the item is old enough, big enough, and matches the masks.
    pub fn accepts(&self, name: &str, size: u64, modified_unix: Option<i64>) -> bool {
        self.accepts_name(name) && self.accepts_age(modified_unix) && self.accepts_size(size)
    }

    pub fn accepts_name(&self, name: &str) -> bool {
        if let Some(include) = &self.include
            && !include.is_match(name)
        {
            return false;
        }
        if let Some(exclude) = &self.exclude
            && exclude.is_match(name)
        {
            return false;
        }
        true
    }

    pub fn accepts_age(&self, modified_unix: Option<i64>) -> bool {
        let Some(days) = self.min_age_days else {
            return true;
        };
        let Some(modified) = modified_unix else {
            return false;
        };
        let age = self.now_unix.saturating_sub(modified);
        age >= i64::from(days) * 86_400
    }

    pub fn accepts_size(&self, size: u64) -> bool {
        self.min_size.is_none_or(|min_size| size >= min_size)
    }

    /// True when a size test needs the measured subtree size.
    pub fn has_size_filter(&self) -> bool {
        self.min_size.is_some()
    }
}

fn compile(masks: &[String]) -> Result<Option<GlobSet>, globset::Error> {
    if masks.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for mask in masks {
        builder.add(
            GlobBuilder::new(mask)
                .case_insensitive(cfg!(windows))
                .build()?,
        );
    }
    Ok(Some(builder.build()?))
}
