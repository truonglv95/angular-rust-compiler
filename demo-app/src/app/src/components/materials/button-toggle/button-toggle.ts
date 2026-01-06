import { Component } from '@angular/core';
import { MatButtonToggleModule } from '@angular/material/button-toggle';
import { MatIconModule } from '@angular/material/icon';
import { FormsModule } from '@angular/forms';

/**
 * @title Button toggle overview
 */
@Component({
  selector: 'button-toggle-overview-example',
  templateUrl: 'button-toggle.html',
  styleUrls: ['button-toggle.css'],
  standalone: true,
  imports: [MatButtonToggleModule, MatIconModule, FormsModule],
})
export class ButtonToggleOverviewExample {}
