use std::str::FromStr;

use crate::schema::v1::schema_client::SchemaClient;
use crate::schema::v1::{SchemaHash, WriteRequest};
use crate::v1::Schema;
use tonic::transport::Channel;

#[derive(Debug, thiserror::Error)]
pub enum WriteSchemaError {
    #[error("request failed: {source}")]
    RequestFailed {
        #[from]
        source: tonic::Status,
    },
    #[error("response from server is missing the schema hash")]
    MissingHash,
}

pub struct Client {
    inner: SchemaClient<Channel>,
    token: Option<String>,
}

impl Client {
    pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<tonic::codegen::StdError>,
    {
        let inner = SchemaClient::connect(dst).await?;
        Ok(Self { inner, token: None })
    }

    /// Sets the authentication token for subsequent server requests.
    pub fn with_token(mut self, token: Option<String>) -> Self {
        self.token = token;
        self
    }

    /// Publishes a compiled IR schema to Aegis.
    /// Returns the Base64-encoded revision digest on success.
    pub async fn write(&mut self, schema: Schema) -> Result<SchemaHash, WriteSchemaError> {
        let mut request = tonic::Request::new(WriteRequest {
            schema: Some(schema),
        });

        if let Some(token) = &self.token
            && let Ok(metadata_val) =
                tonic::metadata::MetadataValue::from_str(&format!("Bearer {token}"))
        {
            request.metadata_mut().insert("authorization", metadata_val);
        }

        let response = self.inner.write(request).await?.into_inner();

        // Extract the digest from the response hash object
        response.hash.ok_or(WriteSchemaError::MissingHash)
    }
}
