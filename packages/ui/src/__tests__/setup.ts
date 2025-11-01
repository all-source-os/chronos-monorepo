import { afterEach, beforeAll } from "bun:test";
import { cleanup } from "@testing-library/react";
import { GlobalRegistrator } from "@happy-dom/global-registrator";

// Register happy-dom globals before any tests run
beforeAll(() => {
  GlobalRegistrator.register();
});

// Cleanup after each test
afterEach(() => {
  cleanup();
});
