use crate::schema::v1::schema_client::SchemaClient;
use crate::schema::v1::{SchemaHash, WriteRequest};
use crate::v1::Schema;
use tonic::transport::Channel;

pub struct Client {
    inner: SchemaClient<Channel>,
}

impl Client {
    pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<tonic::codegen::StdError>,
    {
        let inner = SchemaClient::connect(dst).await?;
        Ok(Self { inner })
    }

    /// Publishes a compiled IR schema to Aegis.
    /// Returns the Base64-encoded revision digest on success.
    pub async fn write(&mut self, schema: Schema) -> Result<SchemaHash, tonic::Status> {
        let request = WriteRequest {
            schema: Some(schema),
        };

        let response = self.inner.write(request).await?.into_inner();

        // Extract the digest from the response hash object
        Ok(response.hash.unwrap_or_default())
    }
}
