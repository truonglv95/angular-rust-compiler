import { Directive, ElementRef, HostListener, Input, Renderer2 } from '@angular/core';
import { NgControl } from '@angular/forms';

@Directive({
  selector: '[appControlFormat]',
})
export class ControlFormat {
  @Input() characterPrevention: RegExp | null = null;

  constructor(private el: ElementRef, private control: NgControl, private renderer2: Renderer2) {}

  @HostListener('input', ['$event.target']) onInput(target: unknown) {
    /* set value on HTML */
    this.renderer2.setValue(this.el.nativeElement, '10');
    /* set value on model */
    this.control.control?.setValue('10');
  }

  @HostListener('paste', ['$event.target']) onPaste(target: unknown) {
    this.renderer2.setValue(this.el.nativeElement, '10');
    this.control.control?.setValue('20');
  }
}
