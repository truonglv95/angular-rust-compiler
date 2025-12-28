import { NgFor } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  EventEmitter,
  inject,
  input,
  Input,
  OnInit,
  output,
  Output,
  signal,
} from '@angular/core';
import { FormBuilder, FormGroup, Validators } from '@angular/forms';
import { RouterOutlet } from '@angular/router';
import { FullNamePipe } from './src/pipes/full-name-pipe';
import { NgForTest } from './src/components/ng-for/ng-for';
import { NgIfTest } from './src/components/ng-if-test/ng-if-test';
import { EventBindingTest } from './src/components/event-binding-test/event-binding-test';
import { PropertyBindingTest } from './src/components/property-binding-test/property-binding-test';
import { TwoWayBindingTest } from './src/components/two-way-binding-test/two-way-binding-test';
import { UnusedImportComponent } from './unused-import.component';
import { AnyTestComponent } from './src/components/any-test/any-test';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [
    RouterOutlet,
    NgFor,
    FullNamePipe,
    NgForTest,
    NgIfTest,
    EventBindingTest,
    PropertyBindingTest,
    TwoWayBindingTest,
    UnusedImportComponent,
    AnyTestComponent,
  ],
  templateUrl: './app.html',
  styleUrl: './app.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class App implements OnInit {
  @Input() fullName = 'Le Van Trường';

  header = input<string>('header 1');

  header2 = input.required<string>();

  @Output() nameChange = new EventEmitter<string>();

  haderChange = output<string>();

  surname = 'Lê';
  name = 'Trường';

  protected readonly title = signal('demo-app 5');
  protected readonly items = signal([
    { title: 'Item 9', link: 'https://example.com/item1' },
    { title: 'Item 2', link: 'https://example.com/item2' },
    { title: 'Item 3', link: 'https://example.com/item3' },
  ]);

  protected readonly fb = inject(FormBuilder);

  protected form!: FormGroup;

  ngOnInit(): void {
    this.form = this.fb.group({
      name: ['string', Validators.required],
    });
  }
}
