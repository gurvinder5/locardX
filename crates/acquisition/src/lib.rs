pub mod artifact;
pub mod destination;
pub mod engine;
pub mod hasher;
pub mod models;
pub mod platform;
pub mod service;
pub mod source;
pub mod writer;

pub use destination::{validate_destination, ValidatedDestination};
pub use engine::{AcquisitionEngine, AcquisitionEngineOutput};
pub use hasher::StreamingSha256Hasher;
pub use models::{
    AcquisitionArtifact, AcquisitionDeviceSnapshot, AcquisitionFailureReason, AcquisitionPlan,
    AcquisitionProgress, AcquisitionResult, AcquisitionStatus,
};
pub use platform::{
    AcquisitionSourceReader, CompositeAcquisitionReader, FaultInjectableAcquisitionReader,
    FaultInjectionMode, ReadOnlyDeviceStream, RealAcquisitionReader,
};
pub use service::AcquisitionService;
pub use source::{create_source_snapshot, revalidate_source_snapshot, validate_source_device_id};
pub use writer::RawImageWriter;
