import {
  Directive,
  Input,
  Output,
  EventEmitter,
  Component,
  model,
  input,
  output,
  ContentChild,
  ViewChild,
  ElementRef,
  OnInit,
  OnChanges,
  Optional,
  Self,
  Attribute,
  HostListener,
  SimpleChanges,
  booleanAttribute,
} from '@angular/core';

@Directive({
  selector: '[host-dir]',
  standalone: true,
})
export class HostDir {
  @Input() hostInput = '';
  @Output() hostOutput = new EventEmitter();
}

@Directive({
  selector: '[all-features]',
  standalone: true,
  hostDirectives: [
    {
      directive: HostDir,
      inputs: ['hostInput: aliasInput'],
      outputs: ['hostOutput: aliasOutput'],
    },
  ],
})
export class AllFeaturesDirective implements OnInit, OnChanges {
  // Inputs
  @Input() regularInput: string = '';
  @Input({ transform: booleanAttribute }) transformInput: boolean = false;

  // Signal Inputs
  sigInput = input<string>('');
  sigInputReq = input.required<number>();

  // Model
  modelInput = model(0);

  // Outputs
  @Output() regularOutput = new EventEmitter<void>();
  sigOutput = output<void>();

  // Queries
  @ContentChild('ref', { read: ElementRef, descendants: true }) contentQuery!: ElementRef;
  @ViewChild('viewRef', { read: ElementRef }) viewQuery!: ElementRef;

  constructor(
    @Optional() @Self() public elementRef: ElementRef,
    @Attribute('staticAttr') public staticAttr: string,
  ) {}

  ngOnInit() {
    console.log('OnInit');
  }

  ngOnChanges(changes: SimpleChanges) {
    console.log('OnChanges', changes);
  }

  @HostListener('click')
  onClick() {
    this.regularOutput.emit();
  }
}
