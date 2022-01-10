use nix::sys::utsname::uname;
use os_release::OsRelease;
use raw_cpuid::CpuId;
use std::{io, process};
use time::{format_description, OffsetDateTime};

mod event;
use report::{report_file, Report, ReportFreq};
mod report;

fn unknown() -> String {
    "unknown".to_string()
}

fn data_header() -> event::TelemetryHeaderModel {
    let (os_name, os_version) = match OsRelease::new() {
        Ok(OsRelease { name, version, .. }) => (name, version),
        Err(_) => (unknown(), unknown()),
    };

    // XXX offset format? Fraction?
    let format =
        format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]").unwrap();
    let (timestamp, timestamp_utc_offset) = match OffsetDateTime::now_local() {
        Ok(time) => (
            time.format(&format).ok().unwrap_or_else(unknown),
            time.offset().whole_hours().into(),
        ),
        Err(_) => (unknown(), 0),
    };

    event::TelemetryHeaderModel {
        consent: event::DataCollectionConsent {
            level: String::new(), // TODO
        },
        data_provider: event::DataProviderInfo {
            app_name: env!("CARGO_PKG_NAME").to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            os_name,
            os_version,
        },
        ids: event::DeviceOSIds {
            bios_uuid: String::new(),     // TODO
            device_id: String::new(),     // TODO
            os_install_id: String::new(), // TODO
        },
        timestamp,
        timestamp_utc_offset,
    }
}

fn main() {
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("hp-vendor: must be run as root");
        process::exit(1);
    }

    let product_name =
        report_file("/sys/class/dmi/id/product_name").unwrap_or_else(|_| "Unknown".to_string());
    if product_name != "HP EliteBook 845 G8 Notebook PC" {
        eprintln!("hp-vendor: unknown product '{}'", product_name);
        process::exit(1);
    }

    let mut report = Report::new();

    {
        let section = report.section("System");
        section.item("Vendor", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/sys_vendor")
        });
        section.item("Family", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/product_family")
        });
        section.item("Name", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/product_name")
        });
        section.item("Serial", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/product_serial")
        });
        section.item("SKU", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/product_sku")
        });
        section.item("Version", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/product_version")
        });
        section.item("UUID", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/product_uuid")
        });
    }

    {
        let section = report.section("Base Board");
        section.item("Vendor", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/board_vendor")
        });
        section.item("Name", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/board_name")
        });
        section.item("Serial", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/board_serial")
        });
        section.item("Version", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/board_version")
        });
    }

    {
        let section = report.section("Firmware");
        section.item("Vendor", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/bios_vendor")
        });
        section.item("Version", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/bios_version")
        });
        section.item("Release Date", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/bios_date")
        });
    }

    {
        let section = report.section("Chassis");
        section.item("Vendor", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/chassis_vendor")
        });
        section.item("Serial", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/chassis_serial")
        });
        section.item("Version", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/chassis_version")
        });
    }

    {
        let section = report.section("Operating System");
        section.item("Name", ReportFreq::Boot, || {
            OsRelease::new().map(|x| x.name)
        });
        section.item("Version", ReportFreq::Boot, || {
            OsRelease::new().map(|x| x.version)
        });
    }

    {
        let section = report.section("Battery");
        section.item("Capacity", ReportFreq::Boot, || {
            report_file("/sys/class/power_supply/BAT0/capacity")
        });
        section.item("Cycle Count", ReportFreq::Boot, || {
            report_file("/sys/class/power_supply/BAT0/cycle_count")
        });
        section.item("Manufacturer", ReportFreq::Boot, || {
            report_file("/sys/class/power_supply/BAT0/manufacturer")
        });
        section.item("Model", ReportFreq::Boot, || {
            report_file("/sys/class/power_supply/BAT0/model_name")
        });
        section.item("Serial", ReportFreq::Boot, || {
            report_file("/sys/class/power_supply/BAT0/serial_number")
        });
        section.item("Technology", ReportFreq::Boot, || {
            report_file("/sys/class/power_supply/BAT0/technology")
        });
    }

    {
        let section = report.section("CPU");
        section.item("Vendor", ReportFreq::Boot, || {
            CpuId::new()
                .get_vendor_info()
                .map(|x| x.as_str().to_string())
                .ok_or(io::Error::new(io::ErrorKind::NotFound, "no cpuid vendor"))
        });
        section.item("Model", ReportFreq::Boot, || {
            CpuId::new()
                .get_processor_brand_string()
                .map(|x| x.as_str().to_string())
                .ok_or(io::Error::new(io::ErrorKind::NotFound, "no cpuid model"))
        });
    }

    report.update();

    println!("{:#?}", report.values());

    let utsname = uname();
    println!(
        "{}",
        serde_json::to_string_pretty(&event::Event {
            data_header: data_header(),
            data: vec![event::AnyTelemetryEventEnum::SwLinuxKernel(
                event::LinuxKernel {
                    name: utsname.sysname().to_string(),
                    release: utsname.release().to_string(),
                    state: event::Swstate::Same, // TODO
                    version: utsname.version().to_string(),
                }
            )]
        })
        .unwrap()
    );
}
