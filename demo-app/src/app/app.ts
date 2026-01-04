import { Component } from '@angular/core';
import { MatToolbarModule } from '@angular/material/toolbar';
import { MatButtonModule } from '@angular/material/button';
import { NavbarComponent } from './src/layout/navbar/navbar';
import { ComponentSidenavComponent } from './src/layout/sidenav/component-sidenav';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [MatToolbarModule, MatButtonModule, NavbarComponent, ComponentSidenavComponent],
  templateUrl: './app.html',
  styleUrl: './app.css',
})
export class App {
  title = 'demo-app';
}
