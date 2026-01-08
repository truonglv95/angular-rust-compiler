import { Component } from '@angular/core';
import { MatTooltipModule } from '@angular/material/tooltip';
import { MatButtonModule } from '@angular/material/button';

/**
 * @title Tooltip
 */
@Component({
  selector: 'tooltip-test',
  styleUrls: ['tooltip.css'],
  templateUrl: 'tooltip.html',
  standalone: true,
  imports: [MatTooltipModule, MatButtonModule],
})
export class TooltipTestComponent {}
