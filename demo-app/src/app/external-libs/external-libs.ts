import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { NgxChartsModule } from '@swimlane/ngx-charts';
import { ToastrService } from 'ngx-toastr';

import { AgGridAngular } from 'ag-grid-angular';
import { ColDef, ModuleRegistry, ClientSideRowModelModule } from 'ag-grid-community';
import { ButtonModule } from 'primeng/button';
import { TranslateService, TranslateModule } from '@ngx-translate/core';
import { NgxSkeletonLoaderModule } from 'ngx-skeleton-loader';

ModuleRegistry.registerModules([ClientSideRowModelModule]);

@Component({
  selector: 'app-external-libs',
  standalone: true,
  imports: [
    CommonModule,
    NgxChartsModule,
    AgGridAngular,
    ButtonModule,
    TranslateModule,
    NgxSkeletonLoaderModule,
  ],
  templateUrl: './external-libs.html',
  styleUrl: './external-libs.css',
})
export class ExternalLibsComponent {
  toastr = inject(ToastrService);
  translate = inject(TranslateService);

  constructor() {
    this.translate.setTranslation('en', {
      HELLO: 'Hello from ngx-translate!',
    });
    this.translate.setDefaultLang('en');
    this.translate.use('en');
  }

  single = [
    {
      name: 'Germany',
      value: 8940000,
    },
    {
      name: 'USA',
      value: 5000000,
    },
    {
      name: 'France',
      value: 7200000,
    },
  ];

  view: [number, number] = [700, 400];

  // options
  showXAxis = true;
  showYAxis = true;
  gradient = false;
  showLegend = true;
  showXAxisLabel = true;
  xAxisLabel = 'Country';
  showYAxisLabel = true;
  yAxisLabel = 'Population';

  colorScheme: any = {
    domain: ['#5AA454', '#E44D25', '#CFC0BB', '#7aa3e5', '#a8385d', '#aae3f5'],
  };

  showToast() {
    this.toastr.success('Hello world!', 'Toastr fun!');
  }

  // AgGrid
  colDefs: ColDef[] = [{ field: 'make' }, { field: 'model' }, { field: 'price' }];

  rowData = [
    { make: 'Tesla', model: 'Model Y', price: 64950 },
    { make: 'Ford', model: 'F-Series', price: 33850 },
    { make: 'Toyota', model: 'Corolla', price: 29600 },
  ];

  // PrimeNG
  loading = false;

  load() {
    this.loading = true;
    setTimeout(() => {
      this.loading = false;
    }, 2000);
  }
}
// touch
