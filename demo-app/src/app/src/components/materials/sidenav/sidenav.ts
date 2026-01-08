import { Component } from '@angular/core';
import { MatSidenavModule } from '@angular/material/sidenav';
import { MatButtonModule } from '@angular/material/button';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'sidenav-test',
  templateUrl: 'sidenav.html',
  styleUrls: ['sidenav.css'],
  standalone: true,
  imports: [MatSidenavModule, MatButtonModule, CommonModule],
})
export class SidenavTestComponent {
  showFiller = false;
}
