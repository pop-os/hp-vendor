// SPDX-FileCopyrightText: 2022 Hewlett-Packard Development Company, L.P.
//
// SPDX-License-Identifier: GPL-3.0-only

use hp_vendor::event;

fn main() {
    for i in event::TelemetryEventType::iter() {
        if hp_vendor::event(i).is_none() {
            println!("{:?}", i);
        }
    }
}
