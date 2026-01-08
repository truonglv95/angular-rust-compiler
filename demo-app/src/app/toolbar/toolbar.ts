import { Component } from '@angular/core';
import { MatToolbarModule } from '@angular/material/toolbar';
import { MatIconModule } from '@angular/material/icon';
import { MatButtonModule } from '@angular/material/button';

/**
 * @title Toolbar
 */
@Component({
  selector: 'toolbar-test',
  styleUrls: ['toolbar.css'],
  templateUrl: 'toolbar.html',
  standalone: true,
  imports: [MatToolbarModule, MatIconModule, MatButtonModule],
})
export class ToolbarTestComponent {}
