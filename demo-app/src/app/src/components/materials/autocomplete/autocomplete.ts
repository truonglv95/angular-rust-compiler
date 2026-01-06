import { Component, ViewChild, AfterViewInit, Directive } from '@angular/core';
import { FormControl, FormsModule, ReactiveFormsModule } from '@angular/forms';
import { Observable } from 'rxjs';
import { map, startWith } from 'rxjs/operators';
import { MatSlideToggleModule } from '@angular/material/slide-toggle';
import { AsyncPipe } from '@angular/common';
import { MatAutocompleteModule } from '@angular/material/autocomplete';
import { MatInputModule } from '@angular/material/input';
import { MatFormFieldModule, MatLabel } from '@angular/material/form-field';

@Component({
  selector: 'app-autocomplete-test',
  templateUrl: 'autocomplete.html',
  styleUrl: 'autocomplete.css',
  standalone: true,
  imports: [
    FormsModule,
    MatFormFieldModule,
    MatInputModule,
    MatAutocompleteModule,
    ReactiveFormsModule,
    MatSlideToggleModule,
    AsyncPipe,
  ],
})
export class AutocompleteTestComponent implements AfterViewInit {
  myControl = new FormControl('');
  options: string[] = ['One', 'Two', 'Three'];
  user: any = { name: 'Nes Nguyen' };
  filteredOptions!: Observable<string[]>;

  @ViewChild(MatLabel) debugLabel: MatLabel | undefined;

  ngAfterViewInit() {
    console.log('[DEBUG] AutocompleteTestComponent found MatLabel:', this.debugLabel);
  }

  ngOnInit() {
    this.filteredOptions = this.myControl.valueChanges.pipe(
      startWith(''),
      map((value) => this._filter(value || '')),
    );
  }

  private _filter(value: string): string[] {
    const filterValue = value.toLowerCase();

    return this.options.filter((option) => option.toLowerCase().includes(filterValue));
  }
}
