/**
 * Shared field validation for account forms.
 *
 * Rules mirror the Rust API: usernames are 3–30 letters, numbers, or
 * underscores; emails follow the same lenient policy the backend enforces.
 * Returning `undefined` means the value is valid.
 */

export const isValidEmail = (value: string): boolean => {
  const atIndex = value.indexOf("@");
  if (atIndex <= 0 || atIndex !== value.lastIndexOf("@")) {
    return false;
  }
  const domain = value.slice(atIndex + 1);
  return (
    domain.includes(".") &&
    !domain.startsWith(".") &&
    !domain.endsWith(".") &&
    !value.includes(" ")
  );
};

export const MIN_USERNAME_LENGTH = 3;
export const MAX_USERNAME_LENGTH = 30;
const USERNAME_CHARS = /^[a-zA-Z0-9_]+$/u;

/** Returns an error message for an invalid username, or `undefined`. */
export const validateUsername = (value: string): string | undefined => {
  const trimmed = value.trim();
  if (!trimmed) {
    return "Username is required";
  }
  if (
    trimmed.length < MIN_USERNAME_LENGTH ||
    trimmed.length > MAX_USERNAME_LENGTH
  ) {
    return `Username must be between ${MIN_USERNAME_LENGTH} and ${MAX_USERNAME_LENGTH} characters`;
  }
  if (!USERNAME_CHARS.test(trimmed)) {
    return "Username may only contain letters, numbers, and underscores";
  }
  return undefined;
};

/** Returns an error message for an invalid email, or `undefined`. */
export const validateEmail = (value: string): string | undefined => {
  const trimmed = value.trim();
  if (!trimmed) {
    return "Email is required";
  }
  if (!isValidEmail(trimmed)) {
    return "Enter a valid email address";
  }
  return undefined;
};
