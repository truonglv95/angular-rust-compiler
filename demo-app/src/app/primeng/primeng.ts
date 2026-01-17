import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ButtonModule } from 'primeng/button';
import { DatePickerModule } from 'primeng/datepicker';
import { FormsModule } from '@angular/forms';
import { CardModule } from 'primeng/card';
import { InputTextModule } from 'primeng/inputtext';
import { InputNumberModule } from 'primeng/inputnumber';
import { CheckboxModule } from 'primeng/checkbox';
import { RadioButtonModule } from 'primeng/radiobutton';
import { SelectModule } from 'primeng/select';
import { MultiSelectModule } from 'primeng/multiselect';
import { TextareaModule } from 'primeng/textarea';
import { ToggleButtonModule } from 'primeng/togglebutton';
import { ToggleSwitchModule } from 'primeng/toggleswitch';
import { RatingModule } from 'primeng/rating';
import { SliderModule } from 'primeng/slider';
import { KnobModule } from 'primeng/knob';
import { ColorPickerModule } from 'primeng/colorpicker';
import { FileUploadModule } from 'primeng/fileupload';
import { AutoCompleteModule } from 'primeng/autocomplete';
import { BadgeModule } from 'primeng/badge';
import { TagModule } from 'primeng/tag';
import { ChipModule } from 'primeng/chip';
import { ProgressBarModule } from 'primeng/progressbar';
import { ProgressSpinnerModule } from 'primeng/progressspinner';
import { AvatarModule } from 'primeng/avatar';
import { AccordionModule } from 'primeng/accordion';
import { PanelModule } from 'primeng/panel';
import { FieldsetModule } from 'primeng/fieldset';
import { DividerModule } from 'primeng/divider';
import { ScrollPanelModule } from 'primeng/scrollpanel';
import { TabsModule } from 'primeng/tabs';
import { ConfirmDialogModule } from 'primeng/confirmdialog';
import { DialogModule } from 'primeng/dialog';
import { ToastModule } from 'primeng/toast';
import { TooltipModule } from 'primeng/tooltip';
import { MenuModule } from 'primeng/menu';
import { MenubarModule } from 'primeng/menubar';
import { TableModule } from 'primeng/table';

@Component({
  selector: 'app-primeng',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    ButtonModule,
    DatePickerModule,
    CardModule,
    InputTextModule,
    InputNumberModule,
    CheckboxModule,
    RadioButtonModule,
    SelectModule,
    MultiSelectModule,
    TextareaModule,
    ToggleButtonModule,
    ToggleSwitchModule,
    RatingModule,
    SliderModule,
    KnobModule,
    ColorPickerModule,
    FileUploadModule,
    AutoCompleteModule,
    BadgeModule,
    TagModule,
    ChipModule,
    ProgressBarModule,
    ProgressSpinnerModule,
    AvatarModule,
    AccordionModule,
    PanelModule,
    FieldsetModule,
    DividerModule,
    ScrollPanelModule,
    TabsModule,
    ConfirmDialogModule,
    DialogModule,
    ToastModule,
    TooltipModule,
    MenuModule,
    MenubarModule,
    TableModule,
  ],
  templateUrl: './primeng.html',
  styleUrl: './primeng.css',
})
export class PrimengTestComponent {
  date: Date | undefined;
  text: string = '';
  numberVal: number = 0;
  checked: boolean = false;
  selectedRadio: string = '';
  selectedItem: any;
  selectedItems: any[] = [];
  ratingVal: number = 3;
  sliderVal: number = 50;
  knobVal: number = 60;
  color: string = '#1976D2';
  switchVal: boolean = false;
  items = [
    { label: 'Option 1', value: '1' },
    { label: 'Option 2', value: '2' },
    { label: 'Option 3', value: '3' },
  ];
  menuItems = [
    { label: 'Home', icon: 'pi pi-home' },
    { label: 'Settings', icon: 'pi pi-cog' },
  ];
  tableData = [
    { id: 1, name: 'Product A', price: 100 },
    { id: 2, name: 'Product B', price: 200 },
  ];
  dialogVisible: boolean = false;

  clickMe() {
    console.log('PrimeNG Button Clicked!');
  }
}
