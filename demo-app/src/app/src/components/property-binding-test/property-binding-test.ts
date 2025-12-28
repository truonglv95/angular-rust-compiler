import { NgIf, NgFor, NgClass, NgStyle } from '@angular/common';
import { Component } from '@angular/core';

@Component({
  selector: 'app-property-binding-test',
  standalone: true,
  imports: [NgIf, NgFor, NgClass, NgStyle],
  templateUrl: './property-binding-test.html',
  styleUrl: './property-binding-test.css',
})
export class PropertyBindingTest {
  // Text bindings
  title = 'Property Binding Demo';
  description = 'Testing various property bindings';

  // Attribute bindings
  imageSrc = 'https://via.placeholder.com/150';
  imageAlt = 'Placeholder Image';
  linkHref = 'https://angular.io';

  // Boolean properties
  isDisabled = false;
  isReadonly = false;
  isHidden = false;

  // Style bindings
  textColor = 'blue';
  fontSize = 16;
  backgroundColor = '#f0f0f0';
  borderRadius = 8;

  // Class bindings
  isActive = false;
  isHighlighted = false;
  isPrimary = true;

  // Dynamic class object
  get dynamicClasses() {
    return {
      active: this.isActive,
      highlighted: this.isHighlighted,
      primary: this.isPrimary,
    };
  }

  // Dynamic style object
  get dynamicStyles() {
    return {
      color: this.textColor,
      'font-size': `${this.fontSize}px`,
      'background-color': this.backgroundColor,
      'border-radius': `${this.borderRadius}px`,
      padding: '10px',
    };
  }

  // Width/Height bindings
  boxWidth = 200;
  boxHeight = 100;

  // ARIA bindings
  ariaLabel = 'Interactive button';
  ariaExpanded = false;
  ariaDisabled = false;

  // Data attributes
  dataId = '12345';
  dataType = 'example';

  // Methods
  toggleDisabled(): void {
    this.isDisabled = !this.isDisabled;
  }

  toggleReadonly(): void {
    this.isReadonly = !this.isReadonly;
  }

  toggleHidden(): void {
    this.isHidden = !this.isHidden;
  }

  toggleActive(): void {
    this.isActive = !this.isActive;
  }

  toggleHighlighted(): void {
    this.isHighlighted = !this.isHighlighted;
  }

  togglePrimary(): void {
    this.isPrimary = !this.isPrimary;
  }

  setColor(color: string): void {
    this.textColor = color;
  }

  increaseFontSize(): void {
    this.fontSize += 2;
  }

  decreaseFontSize(): void {
    if (this.fontSize > 8) {
      this.fontSize -= 2;
    }
  }

  toggleAriaExpanded(): void {
    this.ariaExpanded = !this.ariaExpanded;
  }
}
