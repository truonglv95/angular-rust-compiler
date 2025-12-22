import { provideBrowserGlobalErrorListeners, provideZoneChangeDetection } from "@angular/core";
import { provideRouter } from "@angular/router";
import { routes } from "./app.routes";
export const appConfig = { providers: [
	provideZoneChangeDetection({ eventCoalescing: true }),
	provideBrowserGlobalErrorListeners(),
	provideRouter(routes)
] };
