use serde::{Deserialize, Serialize};

schemafy::schemafy!("event_package.json");

// Unlike genereated binding, an `enum` rather than a `struct`
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnyTelemetryEventEnum {
    HwBaseBoard(BaseBoard),
    HwBattery(Battery),
    HwBatteryUsage(BatteryUsage),
    HwDisplay(Display),
    HwGraphicsCard(GraphicsCard),
    HwMemoryPhysical(MemoryPhysical),
    HwNetworkCard(NetworkCard),
    HwNvmeSmartLog(NvmesmartLog),
    HwNvmeStorageLogical(NvmestorageLogical),
    HwNvmeStoragePhysical(NvmestoragePhysical),
    HwPeripheralAudioPort(PeripheralAudioPort),
    HwPeripheralHdmi(PeripheralHDMI),
    HwPeripheralSimCard(PeripheralSIMCard),
    HwPeripheralUsbTypeA(PeripheralUSBTypeA),
    HwPeripheralUsbTypeC(PeripheralUSBTypeC),
    HwPeripheralUsbTypeCDisplayPort(PeripheralUSBTypeCDisplayPort),
    HwProcessor(Processor),
    HwSystem(System),
    HwThermalContext(ThermalContext),
    HwTpm(TrustedPlatformModule),
    SwDriver(Driver),
    SwFirmware(Firmware),
    SwLinuxDriverCrash(LinuxDriverCrash),
    SwLinuxKernel(LinuxKernel),
    SwOperatingSystem(OperatingSystem),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Event {
    pub data: Vec<AnyTelemetryEventEnum>,
    pub data_header: TelemetryHeaderModel,
}
