// The screen every web stack ships (npm run build:runtime): the screen
// package's host, which reads each window's markup when the application
// starts (DESIGN 7.8).
import { startScreen } from "@clarity/ui/host/start.ts";

startScreen();
