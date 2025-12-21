import { NgFor } from '@angular/common';
import { Component, signal } from '@angular/core';
import { RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, NgFor],
  templateUrl: './app.html',
  styleUrl: './app.css',
})
export class App {
  protected readonly title = signal('demo-app 5');
  protected readonly items = signal([
    { title: 'Item 1', link: 'https://example.com/item1' },
    { title: 'Item 2', link: 'https://example.com/item2' },
    { title: 'Item 3', link: 'https://example.com/item3' },
  ]);
}
