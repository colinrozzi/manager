pub mod frontend;
pub mod build;
pub mod handlers;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelType {
    Frontend,
    Build {
        operation_id: String,
    },
    Programmer {
        operation_id: String,
    },
    Unknown,
}
