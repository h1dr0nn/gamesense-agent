import { describe, it, expect } from "vitest";
import { getDeviceStatusText, getStatusColorClass } from "./index";
import type { DeviceStatus } from "./index";

describe("getDeviceStatusText", () => {
  it("returns Connected for Device status", () => {
    expect(getDeviceStatusText("Device")).toBe("Connected");
  });

  it("returns Offline for Offline status", () => {
    expect(getDeviceStatusText("Offline")).toBe("Offline");
  });

  it("returns Unauthorized for Unauthorized status", () => {
    expect(getDeviceStatusText("Unauthorized")).toBe("Unauthorized");
  });

  it("returns the Unknown value for Unknown status", () => {
    const status: DeviceStatus = { Unknown: "recovery" };
    expect(getDeviceStatusText(status)).toBe("recovery");
  });
});

describe("getStatusColorClass", () => {
  it("returns status-connected for Device", () => {
    expect(getStatusColorClass("Device")).toBe("status-connected");
  });

  it("returns status-offline for Offline", () => {
    expect(getStatusColorClass("Offline")).toBe("status-offline");
  });

  it("returns status-warning for Unauthorized", () => {
    expect(getStatusColorClass("Unauthorized")).toBe("status-warning");
  });

  it("returns status-unknown for Unknown status", () => {
    const status: DeviceStatus = { Unknown: "sideload" };
    expect(getStatusColorClass(status)).toBe("status-unknown");
  });
});
