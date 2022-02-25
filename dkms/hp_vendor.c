// SPDX-License-Identifier: GPL-2.0-or-later
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
#include <linux/types.h>
#include <linux/version.h>

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

/* hwmon */

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
			ec_read(0x2E, &raw);

			if (raw == 0 || raw == 0xFF) {
				*val = 0;
			} else {
				*val = (long)(245760) / (long)(raw);
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

	return 0;
}

static void __exit hp_vendor_exit(void)
{
	platform_device_unregister(hp_vendor_platform_device);
	platform_driver_unregister(&hp_vendor_platform_driver);
}

module_init(hp_vendor_init);
module_exit(hp_vendor_exit);

MODULE_DESCRIPTION("HP Vendor Driver");
MODULE_AUTHOR("Jeremy Soller <jeremy@system76.com>");
MODULE_LICENSE("GPL");
