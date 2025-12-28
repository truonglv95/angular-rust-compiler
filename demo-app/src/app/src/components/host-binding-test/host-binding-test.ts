import { Component, HostBinding, HostListener } from '@angular/core';

@Component({
  selector: 'app-host-binding-test',
  standalone: true,
  template: '<p>Host Binding Test</p>',
  host: {
    id: 'host-id',
    '[class.active]': 'isActive',
    '[style.color]': 'color',
    '(click)': 'onClick($event)',
  },
})
export class HostBindingTest {
  isActive = true;
  color = 'red';

  onClick(event: MouseEvent) {
    console.log('Host clicked', event);
    this.isActive = !this.isActive;
  }
}
