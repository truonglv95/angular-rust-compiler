import { Component } from '@angular/core';
import { MatRippleModule } from '@angular/material/core';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatCheckboxModule } from '@angular/material/checkbox';
import { FormsModule } from '@angular/forms';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'ripple-test',
  templateUrl: 'ripple.html',
  styleUrls: ['ripple.css'],
  standalone: true,
  imports: [
    MatRippleModule,
    MatFormFieldModule,
    MatInputModule,
    MatCheckboxModule,
    FormsModule,
    CommonModule,
  ],
})
export class RippleTestComponent {
  centered = false;
  disabled = false;
  unbounded = false;
  radius: number = 0;
  color: string = '';
}
