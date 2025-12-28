import { Component, Input, Output, EventEmitter } from '@angular/core';
import { NgIf, NgFor, JsonPipe } from '@angular/common';
import { FormsModule } from '@angular/forms';

@Component({
  selector: 'app-two-way-binding-test',
  standalone: true,
  imports: [NgIf, NgFor, FormsModule, JsonPipe],
  templateUrl: './two-way-binding-test.html',
  styleUrl: './two-way-binding-test.css',
})
export class TwoWayBindingTest {
  // Basic two-way binding
  name = 'Angular';
  email = '';
  age = 25;

  // Textarea
  message = 'Hello World!';

  // Select
  selectedCountry = 'vn';
  countries = [
    { code: 'vn', name: 'Vietnam' },
    { code: 'us', name: 'United States' },
    { code: 'jp', name: 'Japan' },
    { code: 'kr', name: 'Korea' },
  ];

  // Checkbox
  agreeTerms = false;
  receiveNewsletter = true;

  // Radio
  gender = 'other';

  // Range slider
  volume = 50;
  brightness = 75;

  // Date/Time
  selectedDate = '';
  selectedTime = '';

  // Color picker
  favoriteColor = '#3f51b5';

  // Computed
  get nameLength(): number {
    return this.name.length;
  }

  get volumeLabel(): string {
    if (this.volume < 30) return 'Low';
    if (this.volume < 70) return 'Medium';
    return 'High';
  }

  // Methods
  resetForm(): void {
    this.name = '';
    this.email = '';
    this.age = 25;
    this.message = '';
    this.selectedCountry = 'vn';
    this.agreeTerms = false;
    this.receiveNewsletter = true;
    this.gender = 'other';
    this.volume = 50;
    this.brightness = 75;
    this.selectedDate = '';
    this.selectedTime = '';
    this.favoriteColor = '#3f51b5';
  }

  submitForm(): void {
    console.log('Form submitted:', {
      name: this.name,
      email: this.email,
      age: this.age,
      message: this.message,
      country: this.selectedCountry,
      agreeTerms: this.agreeTerms,
      newsletter: this.receiveNewsletter,
      gender: this.gender,
    });
  }
}
