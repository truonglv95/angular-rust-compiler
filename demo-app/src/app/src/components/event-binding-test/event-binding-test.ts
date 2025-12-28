import { NgIf, NgFor } from '@angular/common';
import { Component } from '@angular/core';
import { FormsModule } from '@angular/forms';

interface EventLog {
  type: string;
  timestamp: Date;
  details: string;
}

@Component({
  selector: 'app-event-binding-test',
  standalone: true,
  imports: [NgIf, NgFor, FormsModule],
  templateUrl: './event-binding-test.html',
  styleUrl: './event-binding-test.css',
})
export class EventBindingTest {
  // Click events
  clickCount = 0;
  lastClickTime: string = '';

  // Mouse events
  mousePosition = { x: 0, y: 0 };
  isHovering = false;

  // Keyboard events
  inputValue = '';
  lastKeyPressed = '';
  keyPressCount = 0;

  // Focus events
  isFocused = false;

  // Form events
  formValue = '';
  selectedOption = '';
  checkboxValue = false;

  // Event log
  eventLog: EventLog[] = [];
  maxLogEntries = 10;

  // Click handlers
  onClick(): void {
    this.clickCount++;
    this.lastClickTime = new Date().toLocaleTimeString();
    this.logEvent('click', `Button clicked (count: ${this.clickCount})`);
  }

  onDoubleClick(): void {
    this.logEvent('dblclick', 'Double click detected!');
  }

  onRightClick(event: MouseEvent): void {
    event.preventDefault();
    this.logEvent('contextmenu', 'Right click detected!');
  }

  // Mouse handlers
  onMouseEnter(): void {
    this.isHovering = true;
    this.logEvent('mouseenter', 'Mouse entered element');
  }

  onMouseLeave(): void {
    this.isHovering = false;
    this.logEvent('mouseleave', 'Mouse left element');
  }

  onMouseMove(event: MouseEvent): void {
    this.mousePosition = { x: event.clientX, y: event.clientY };
  }

  onMouseDown(event: MouseEvent): void {
    this.logEvent('mousedown', `Mouse button ${event.button} down`);
  }

  onMouseUp(event: MouseEvent): void {
    this.logEvent('mouseup', `Mouse button ${event.button} up`);
  }

  // Keyboard handlers
  onKeyDown(event: KeyboardEvent): void {
    this.lastKeyPressed = event.key;
    this.keyPressCount++;
    this.logEvent('keydown', `Key pressed: ${event.key}`);
  }

  onKeyUp(event: KeyboardEvent): void {
    this.logEvent('keyup', `Key released: ${event.key}`);
  }

  onInput(event: Event): void {
    const target = event.target as HTMLInputElement;
    this.inputValue = target.value;
  }

  onEnterKey(): void {
    this.logEvent('keydown.enter', 'Enter key pressed!');
  }

  onEscapeKey(): void {
    this.inputValue = '';
    this.logEvent('keydown.escape', 'Escape key - input cleared!');
  }

  // Focus handlers
  onFocus(): void {
    this.isFocused = true;
    this.logEvent('focus', 'Input focused');
  }

  onBlur(): void {
    this.isFocused = false;
    this.logEvent('blur', 'Input blurred');
  }

  // Form handlers
  onSubmit(event: Event): void {
    event.preventDefault();
    this.logEvent('submit', `Form submitted with value: ${this.formValue}`);
  }

  onSelectChange(event: Event): void {
    const target = event.target as HTMLSelectElement;
    this.selectedOption = target.value;
    this.logEvent('change', `Selected: ${target.value}`);
  }

  onCheckboxChange(event: Event): void {
    const target = event.target as HTMLInputElement;
    this.checkboxValue = target.checked;
    this.logEvent('change', `Checkbox: ${target.checked ? 'checked' : 'unchecked'}`);
  }

  // Helper methods
  logEvent(type: string, details: string): void {
    this.eventLog.unshift({
      type,
      timestamp: new Date(),
      details,
    });
    if (this.eventLog.length > this.maxLogEntries) {
      this.eventLog.pop();
    }
  }

  clearLog(): void {
    this.eventLog = [];
  }

  resetAll(): void {
    this.clickCount = 0;
    this.lastClickTime = '';
    this.inputValue = '';
    this.lastKeyPressed = '';
    this.keyPressCount = 0;
    this.formValue = '';
    this.selectedOption = '';
    this.checkboxValue = false;
    this.eventLog = [];
  }
}
