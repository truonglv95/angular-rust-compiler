import { NgFor, NgIf } from '@angular/common';
import { ChangeDetectionStrategy, Component } from '@angular/core';

interface User {
  id: number;
  name: string;
  email: string;
  active: boolean;
}

interface Category {
  name: string;
  items: string[];
}

@Component({
  selector: 'app-ng-for',
  standalone: true,
  imports: [NgFor, NgIf],
  templateUrl: './ng-for.html',
  styleUrl: './ng-for.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NgForTest {
  // Basic array
  protected readonly items = ['Item 1', 'Item 2', 'Item 3'];

  // Array of objects
  protected users: User[] = [
    { id: 1, name: 'Alice', email: 'alice@example.com', active: true },
    { id: 2, name: 'Bob', email: 'bob@example.com', active: false },
    { id: 3, name: 'Charlie', email: 'charlie@example.com', active: true },
  ];

  // Nested arrays
  protected categories: Category[] = [
    { name: 'Fruits', items: ['Apple', 'Banana', 'Orange'] },
    { name: 'Vegetables', items: ['Carrot', 'Broccoli', 'Spinach'] },
    { name: 'Dairy', items: ['Milk', 'Cheese', 'Yogurt'] },
  ];

  // Numbers array for index testing
  protected numbers = [10, 20, 30, 40, 50];

  // TrackBy function
  trackByUserId(index: number, user: User): number {
    return user.id;
  }

  trackByIndex(index: number): number {
    return index;
  }

  // Methods for dynamic operations
  addUser(): void {
    const newId = this.users.length + 1;
    this.users = [
      ...this.users,
      { id: newId, name: `User ${newId}`, email: `user${newId}@example.com`, active: true },
    ];
  }

  removeUser(id: number): void {
    this.users = this.users.filter((u) => u.id !== id);
  }

  toggleActive(user: User): void {
    user.active = !user.active;
  }
}
