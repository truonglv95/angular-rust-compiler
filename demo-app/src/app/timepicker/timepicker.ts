import { ChangeDetectionStrategy, Component } from '@angular/core';
import { MatTimepickerModule } from '@angular/material/timepicker';
import { MatInputModule } from '@angular/material/input';
import { MatFormFieldModule } from '@angular/material/form-field';
import { provideNativeDateAdapter } from '@angular/material/core';

/**
 * @title Timepicker
 */
@Component({
  selector: 'timepicker-test',
  styleUrls: ['timepicker.css'],
  templateUrl: 'timepicker.html',
  standalone: true,
  imports: [MatFormFieldModule, MatInputModule, MatTimepickerModule],
  providers: [provideNativeDateAdapter()],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TimepickerTestComponent {}
