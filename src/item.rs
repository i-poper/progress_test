#[derive(Clone, Debug, Default)]
pub struct Item {
    pub name: String,
    pub total: u64,
    pub size: u64,
    pub canceled: bool,
}
