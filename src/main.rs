use os_release::OsRelease;

use report::{Report, ReportFreq, report_file};
mod report;

fn main() {
    let mut report = Report::new();

    {
        let section = report.section("System");
        section.item("Vendor", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/sys_vendor")
        });
        section.item("Name", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/product_name")
        });
        section.item("Version", ReportFreq::Boot, || {
            report_file("/sys/class/dmi/id/product_version")
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
