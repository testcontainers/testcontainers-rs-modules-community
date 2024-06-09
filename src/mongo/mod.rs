mod replset;
mod standalong;

pub use replset::MongoReplSet;
pub use standalong::Mongo;

const NAME: &str = "mongo";
const TAG: &str = "5.0.22";

#[cfg(test)]
static NAME_TAG_VARIANTS: &[(&str, &str)] = &[
    ("mongodb/mongodb-community-server", "7.0.7-ubi8"),
    ("mongodb/mongodb-enterprise-server", "7.0.7-ubi8"),
    ("mongo", "7"),
    ("mongo", "6"),
    ("mongo", "5"),
];
