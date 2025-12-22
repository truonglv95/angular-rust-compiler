import * as i0 from "@angular/core";
import { Directive, HostListener, Input } from "@angular/core";
export class ControlFormat {
	characterPrevention = null;
	constructor(el, control, renderer2) {
		this.el = el;
		this.control = control;
		this.renderer2 = renderer2;
	}
	onInput(target) {
		/* set value on HTML */
		this.renderer2.setValue(this.el.nativeElement, "10");
		/* set value on model */
		this.control.control?.setValue("10");
	}
	onPaste(target) {
		this.renderer2.setValue(this.el.nativeElement, "10");
		this.control.control?.setValue("20");
	}
	static ɵfac = function ControlFormat_Factory(t) {
		return new (t || ControlFormat)();
	};
	static ɵdir = /* @__PURE__ */ i0.ɵɵdefineDirective({
		type: ControlFormat,
		selectors: [["[appControlFormat]"]],
		inputs: { characterPrevention: "characterPrevention" }
	});
}
