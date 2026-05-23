mod client;
mod helpers;

#[allow(clippy::all)]
#[allow(unused_qualifications)]
#[allow(dead_code)]
#[allow(missing_docs)]

pub mod v1 {
    // Includes the core IR types (Schema, TypeDefinition, ActorType, etc.)
    // Path is relative to this lib.rs file
    include!("generated/aegis/v1/aegis.v1.rs");
}

pub mod schema {
    pub mod v1 {
        // Includes the Schema Service, WriteRequest, ReadResponse, etc.
        // This file also automatically includes the .tonic.rs sidecar
        include!("generated/aegis/schema/v1/aegis.schema.v1.rs");
    }
}

pub use client::Client;
