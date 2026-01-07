import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatDividerModule } from '@angular/material/divider';
import { MatListModule } from '@angular/material/list';

@Component({
  selector: 'app-divider-test',
  standalone: true,
  imports: [CommonModule, MatDividerModule, MatListModule],
  templateUrl: './divider.html',
  styleUrls: ['./divider.css'],
})
export class DividerTestComponent {}
