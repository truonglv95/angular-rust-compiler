import { Component } from '@angular/core';
import { MatTabsModule } from '@angular/material/tabs';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'tabs-test',
  templateUrl: 'tabs.html',
  styleUrls: ['tabs.css'],
  standalone: true,
  imports: [MatTabsModule, CommonModule],
})
export class TabsTestComponent {}
