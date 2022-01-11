use indexmap::IndexMap;
use log::error;
use std::{fs, io, path::Path, time::Instant};

pub fn report_file<P: AsRef<Path>>(path: P) -> io::Result<String> {
    fs::read_to_string(path).map(|x| x.trim().to_string())
}

#[derive(Copy, Clone)]
pub enum ReportFreq {
    /// One update per boot cycle
    Boot,
    /// One update per hour, or on each boot
    Hourly,
    /// One update per day, or on each boot
    Daily,
}

pub struct ReportItem {
    /// The name of the report item
    pub name: &'static str,
    /// The frequency the value should update at
    pub freq: ReportFreq,
    /// Function for reading latest value
    pub func: Box<dyn FnMut() -> io::Result<String>>,
    /// The last update value
    pub value: Option<String>,
    /// The last update time
    pub time: Option<Instant>,
}

impl ReportItem {
    /// Update value when requested and return true if value has changed
    pub fn update(&mut self) -> io::Result<bool> {
        let expired = match self.time {
            Some(time) => match self.freq {
                ReportFreq::Boot => false,
                ReportFreq::Hourly => time.elapsed().as_secs() >= 60 * 60,
                ReportFreq::Daily => time.elapsed().as_secs() >= 24 * 60 * 60,
            },
            None => true,
        };
        if expired {
            let new = (self.func)()?;
            let updated = self.value.take().map_or(true, |x| x != new);
            self.value = Some(new);
            self.time = Some(Instant::now());
            Ok(updated)
        } else {
            Ok(false)
        }
    }
}

pub struct ReportSection {
    name: &'static str,
    items: Vec<ReportItem>,
}

impl ReportSection {
    /// Add an item to this report section
    pub fn item(
        &mut self,
        name: &'static str,
        freq: ReportFreq,
        func: impl FnMut() -> io::Result<String> + 'static,
    ) {
        self.items.push(ReportItem {
            name,
            freq,
            func: Box::new(func),
            value: None,
            time: None,
        });
    }
}

pub struct Report {
    pub sections: Vec<ReportSection>,
}

impl Report {
    /// Create a new report
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
        }
    }

    /// Add a report section
    pub fn section(&mut self, name: &'static str) -> &mut ReportSection {
        self.sections.push(ReportSection {
            name: name,
            items: Vec::new(),
        });
        self.sections.last_mut().unwrap()
    }

    /// Update all items and return true if any changed
    pub fn update(&mut self) -> bool {
        let mut updated = false;
        for section in self.sections.iter_mut() {
            for item in section.items.iter_mut() {
                match item.update() {
                    Ok(true) => updated = true,
                    Ok(false) => (),
                    Err(err) => {
                        error!("{}: {}: failed to report: {}", section.name, item.name, err);
                    }
                }
            }
        }
        updated
    }

    /// Read all values of the report
    pub fn values(&self) -> IndexMap<&'static str, IndexMap<&'static str, String>> {
        let mut values = IndexMap::new();
        for section in self.sections.iter() {
            for item in section.items.iter() {
                if let Some(value) = &item.value {
                    values
                        .entry(section.name)
                        .or_insert(IndexMap::new())
                        .insert(item.name, value.clone());
                }
            }
        }
        values
    }
}
