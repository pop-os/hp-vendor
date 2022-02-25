// SPDX-License-Identifier: GPL-2.0-only
/*
 * HP Vendor Driver
 *
 * Copyright (C) 2022 HP
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License version 2 as
 * published by the Free Software Foundation.
 */

#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/acpi.h>
#include <linux/dmi.h>
#include <linux/hwmon.h>
#include <linux/hwmon-sysfs.h>
#include <linux/init.h>
#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/platform_device.h>
#include <linux/power_supply.h>
#include <linux/sysfs.h>
#include <linux/types.h>
#include <linux/version.h>

#include <acpi/battery.h>

static const struct dmi_system_id hp_vendor_dmi_table[] = {
	{
		.ident = "HP Dev One",
		.matches = {
			DMI_MATCH(DMI_BOARD_VENDOR, "HP"),
			DMI_MATCH(DMI_BOARD_NAME, "8A78"),
		},
	},
	{}
};
MODULE_DEVICE_TABLE(dmi, hp_vendor_dmi_table);

/* battery */

#define EC_MAILBOX_PORT_ADDR 0x200
#define EC_MAILBOX_PORT_DATA 0x201
#define EC_MAILBOX_INDEX_CT_NUMBER 0xA1

static unsigned char ec_mailbox_read(uint8_t index) {
	outb(index, EC_MAILBOX_PORT_ADDR);
	return inb(EC_MAILBOX_PORT_DATA);
}

static ssize_t battery_ct_number_show(struct device *dev,
	struct device_attribute *attr, char *buf)
{
	int count;
	for (count = 0; count < 14; count++) {
		buf[count] = ec_mailbox_read(EC_MAILBOX_INDEX_CT_NUMBER + count);
	}
	buf[count++] = '\n';
	buf[count] = 0;
	return count;
}

static DEVICE_ATTR_RO(battery_ct_number);

static struct attribute *hp_vendor_battery_attrs[] = {
	&dev_attr_battery_ct_number.attr,
	NULL,
};

ATTRIBUTE_GROUPS(hp_vendor_battery);

static int hp_vendor_battery_add(struct power_supply *battery)
{
	// HP vendor only supports 1 battery
	if (strcmp(battery->desc->name, "BATT") != 0)
		return -ENODEV;

	if (device_add_groups(&battery->dev, hp_vendor_battery_groups))
		return -ENODEV;

	return 0;
}

static int hp_vendor_battery_remove(struct power_supply *battery)
{
	device_remove_groups(&battery->dev, hp_vendor_battery_groups);
	return 0;
}

static struct acpi_battery_hook hp_vendor_battery_hook = {
	.add_battery = hp_vendor_battery_add,
	.remove_battery = hp_vendor_battery_remove,
	.name = "HP Vendor Battery Extension",
};

/* hwmon */

#define EC_INDEX_FAN_SPEED 0x2E
#define EC_FAN_SPEED_MODIFIER 245760

static umode_t thermal_is_visible(const void *drvdata, enum hwmon_sensor_types type,
				  u32 attr, int channel)
{
	switch (type) {
	case hwmon_fan:
		switch (channel) {
		case 0:
			return 0444;
		default:
			break;
		}
		break;
	default:
		break;
	}
	return 0;
}

static int thermal_read(struct device *dev, enum hwmon_sensor_types type, u32 attr,
			int channel, long *val)
{
	switch (type) {
	case hwmon_fan:
		switch (channel) {
		case 0:
			u8 raw;
			ec_read(EC_INDEX_FAN_SPEED, &raw);

			if (raw == 0 || raw == 0xFF) {
				*val = 0;
			} else {
				*val = (long)(EC_FAN_SPEED_MODIFIER) / (long)(raw);
			}
			return 0;
		default:
			break;
		}
		break;
	default:
		break;
	}
	return -EOPNOTSUPP;
}

static int thermal_read_string(struct device *dev, enum hwmon_sensor_types type, u32 attr,
			       int channel, const char **str)
{
	switch (type) {
	case hwmon_fan:
		switch (channel) {
		case 0:
			*str = "CPU FAN";
			return 0;
		default:
			break;
		}
		break;
	default:
		break;
	}
	return -EOPNOTSUPP;
}

static const struct hwmon_ops thermal_ops = {
	.is_visible = thermal_is_visible,
	.read = thermal_read,
	.read_string = thermal_read_string,
};

static const struct hwmon_channel_info *thermal_channel_info[] = {
	HWMON_CHANNEL_INFO(fan, HWMON_F_INPUT | HWMON_F_LABEL),
	NULL
};

static const struct hwmon_chip_info thermal_chip_info = {
	.ops = &thermal_ops,
	.info = thermal_channel_info,
};

/* platform */

static struct platform_driver hp_vendor_platform_driver = {
	.driver = {
		.name  = "hp_vendor",
		.owner = THIS_MODULE,
	},
};

static struct platform_device *hp_vendor_platform_device = NULL;
static struct device *hp_vendor_hwmon = NULL;

static int __init hp_vendor_init(void)
{
	if (dmi_check_system(hp_vendor_dmi_table)) {
		pr_info("Found supported system");
	} else {
		pr_info("System does not need this driver");
		return -ENODEV;
	}

	hp_vendor_platform_device =
		platform_create_bundle(&hp_vendor_platform_driver, NULL, NULL, 0, NULL, 0);
	if (IS_ERR(hp_vendor_platform_device)) {
		return PTR_ERR(hp_vendor_platform_device);
	}

	hp_vendor_hwmon = devm_hwmon_device_register_with_info(&hp_vendor_platform_device->dev,
		"hp_vendor", NULL, &thermal_chip_info, NULL);
	if (PTR_ERR_OR_ZERO(hp_vendor_hwmon)) {
		platform_device_unregister(hp_vendor_platform_device);
		platform_driver_unregister(&hp_vendor_platform_driver);

		return PTR_ERR_OR_ZERO(hp_vendor_hwmon);
	}

	battery_hook_register(&hp_vendor_battery_hook);

	return 0;
}

static void __exit hp_vendor_exit(void)
{
	battery_hook_unregister(&hp_vendor_battery_hook);

	platform_device_unregister(hp_vendor_platform_device);
	platform_driver_unregister(&hp_vendor_platform_driver);
}

module_init(hp_vendor_init);
module_exit(hp_vendor_exit);

MODULE_DESCRIPTION("HP Vendor Driver");
MODULE_AUTHOR("Jeremy Soller <jeremy@system76.com>");
MODULE_LICENSE("GPL");
