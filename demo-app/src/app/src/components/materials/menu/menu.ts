import { Component } from '@angular/core';
import { MatMenuModule } from '@angular/material/menu';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';

@Component({
  selector: 'app-menu',
  standalone: true,
  imports: [MatMenuModule, MatButtonModule, MatIconModule],
  templateUrl: './menu.html',
  styleUrl: './menu.css',
})
export class MenuTestComponent {}
