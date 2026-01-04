import { Component, ChangeDetectionStrategy } from '@angular/core';
import { MatSidenavModule } from '@angular/material/sidenav';
import { MatListModule } from '@angular/material/list';
import { RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-component-sidenav',
  standalone: true,
  imports: [MatSidenavModule, MatListModule, RouterLink, RouterLinkActive, RouterOutlet],
  templateUrl: './component-sidenav.html',
  styleUrl: './component-sidenav.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ComponentSidenavComponent {}
