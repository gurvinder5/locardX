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
pub use engine::{
    AcquisitionEngine, AcquisitionEngineError, AcquisitionEngineOptions, AcquisitionEngineOutput,
};
pub use hasher::StreamingSha256Hasher;
pub use models::{
    AcquisitionArtifact, AcquisitionDeviceSnapshot, AcquisitionDiagnostics,
    AcquisitionFailureReason, AcquisitionPerformanceStats, AcquisitionPlan, AcquisitionProgress,
    AcquisitionRecordDto, AcquisitionResult, AcquisitionStatus, ArtifactCleanupStatus,
};
pub use platform::{
    AcquisitionSourceReader, CompositeAcquisitionReader, FaultInjectableAcquisitionReader,
    FaultInjectionMode, ReadOnlyDeviceStream, RealAcquisitionReader,
};
pub use service::{
    validate_chunk_size, AcquisitionService, DEFAULT_CHUNK_SIZE_BYTES, SUPPORTED_CHUNK_SIZES,
};
pub use source::{create_source_snapshot, revalidate_source_snapshot, validate_source_device_id};
pub use writer::{
    delete_artifact_verified, staging_path_for, ArtifactCleanupResult, RawImageWriter,
};
