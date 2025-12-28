import { NgIf } from '@angular/common';
import { Component } from '@angular/core';

type UserRole = 'admin' | 'user' | 'guest';

interface UserProfile {
  name: string;
  role: UserRole;
  premium: boolean;
}

@Component({
  selector: 'app-ng-if-test',
  standalone: true,
  imports: [NgIf],
  templateUrl: './ng-if-test.html',
  styleUrl: './ng-if-test.css',
})
export class NgIfTest {
  // Basic boolean
  protected isShow = true;
  protected isLoggedIn = false;
  protected isLoading = false;

  // Nullable values
  protected userName: string | null = 'John Doe';
  protected errorMessage: string | null = null;

  // Numeric conditions
  protected count = 0;
  protected maxCount = 5;

  // Object for complex conditions
  protected user: UserProfile | null = {
    name: 'Admin User',
    role: 'admin',
    premium: true,
  };

  // Array for empty check
  protected items: string[] = ['Item 1', 'Item 2'];

  // Methods
  toggleShow(): void {
    this.isShow = !this.isShow;
  }

  toggleLogin(): void {
    this.isLoggedIn = !this.isLoggedIn;
    if (this.isLoggedIn) {
      this.userName = 'John Doe';
    } else {
      this.userName = null;
    }
  }

  simulateLoading(): void {
    this.isLoading = true;
    this.errorMessage = null;
    setTimeout(() => {
      this.isLoading = false;
    }, 2000);
  }

  simulateError(): void {
    this.errorMessage = 'Something went wrong! Please try again.';
  }

  clearError(): void {
    this.errorMessage = null;
  }

  increment(): void {
    if (this.count < this.maxCount) {
      this.count++;
    }
  }

  decrement(): void {
    if (this.count > 0) {
      this.count--;
    }
  }

  toggleUser(): void {
    if (this.user) {
      this.user = null;
    } else {
      this.user = { name: 'Admin User', role: 'admin', premium: true };
    }
  }

  setUserRole(role: UserRole): void {
    if (this.user) {
      this.user = { ...this.user, role };
    }
  }

  togglePremium(): void {
    if (this.user) {
      this.user = { ...this.user, premium: !this.user.premium };
    }
  }

  addItem(): void {
    this.items = [...this.items, `Item ${this.items.length + 1}`];
  }

  clearItems(): void {
    this.items = [];
  }
}
