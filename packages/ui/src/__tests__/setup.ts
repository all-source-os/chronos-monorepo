import { afterEach, beforeAll, expect } from "bun:test";
import { GlobalRegistrator } from "@happy-dom/global-registrator";
import * as matchers from "@testing-library/jest-dom/matchers";
import { cleanup } from "@testing-library/react";

// Extend Bun's expect with jest-dom matchers for DOM assertions
expect.extend(matchers);

// Register happy-dom globals before any tests run
beforeAll(() => {
  GlobalRegistrator.register();
});

// Cleanup after each test
afterEach(() => {
  cleanup();
});
