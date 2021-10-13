use os_release::OsRelease;
use std::process;

use report::{Report, ReportFreq, report_file};
mod report;

fn main() {
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("hp-vendor: must be run as root");
        process::exit(1);
    }

    let sys_vendor = report_file("/sys/class/dmi/id/sys_vendor").unwrap_or_else(|_| {
        "Unknown".to_string()
    });
    if sys_vendor != "HP" {
        eprintln!("hp-vendor: unknown vendor '{}'", sys_vendor);
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

    report.update();

    println!("{:#?}", report.values());
}
