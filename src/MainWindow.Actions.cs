using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Threading;

namespace LocalPass
{
    internal sealed partial class MainWindow
    {
        private void WireEvents()
        {
            SourceInitialized += delegate
            {
                WindowInteropHelper helper = new WindowInteropHelper(this);
                _clipboard = new SensitiveClipboard(delegate { return helper.Handle; });
            };

            Loaded += delegate
            {
                _active = true;
                _shell.Opacity = 1;
                RefreshRows();
            };
            Activated += delegate
            {
                _active = true;
                _rollTimer.Stop();
                Unroll();
                _shell.Opacity = 1;
                RefreshRows();
            };
            Deactivated += delegate
            {
                _active = false;
                RefreshRows(false);
                if (_infoPopup.IsOpen)
                    return;
                if (IsMouseOver)
                {
                    _rollTimer.Stop();
                    Unroll();
                    if (!Theme.HighContrast)
                        _shell.Opacity = 0.80;
                }
                else
                {
                    ScheduleRollUp();
                    if (!Theme.HighContrast)
                        _shell.Opacity = 0.50;
                }
            };
            MouseEnter += delegate
            {
                _rollTimer.Stop();
                Unroll();
                if (!_active && !Theme.HighContrast)
                    _shell.Opacity = 0.80;
            };
            MouseLeave += delegate
            {
                if (!_active && !_infoPopup.IsOpen)
                {
                    ScheduleRollUp();
                    if (!Theme.HighContrast)
                        _shell.Opacity = 0.50;
                }
            };
            _infoPopup.Opened += delegate { _rollTimer.Stop(); };
            _infoPopup.Closed += delegate
            {
                if (!_active && !IsMouseOver)
                    ScheduleRollUp();
            };
            _rollTimer.Tick += delegate
            {
                _rollTimer.Stop();
                if (!_active && !IsMouseOver && !_infoPopup.IsOpen && !_pendingClose)
                    RollUp();
            };
            _length.ValueChanged += delegate { _lengthValue.Text = ((int)_length.Value).ToString(); };
            _lengthValue.PreviewTextInput += NumericPreviewTextInput;
            _lengthValue.PreviewKeyDown += NumericKeyDown;
            _lengthValue.LostKeyboardFocus += delegate { ReadLength(); };
            DataObject.AddPastingHandler(_lengthValue, NumericPaste);

            _count.PreviewTextInput += NumericPreviewTextInput;
            _count.PreviewKeyDown += NumericKeyDown;
            _count.LostKeyboardFocus += delegate { ReadCount(); };
            DataObject.AddPastingHandler(_count, NumericPaste);

            _generate.Click += delegate { GeneratePasswords(); };
            _clear.Click += delegate { ClearAll(); };
            _infoButton.Click += delegate { _infoPopup.IsOpen = !_infoPopup.IsOpen; };
            _theme.Click += delegate { ToggleTheme(); };
            _close.Click += delegate { Close(); };
            _mask.Checked += delegate { SetMask(true); };
            _mask.Unchecked += delegate { SetMask(false); };
            _pin.Checked += delegate { Topmost = true; };
            _pin.Unchecked += delegate { Topmost = false; };
            _clipMinus.Click += delegate { ChangeClipboardSeconds(-5); };
            _clipPlus.Click += delegate { ChangeClipboardSeconds(5); };

            _clipboardTimer.Tick += ClipboardTick;
            _statusHideTimer.Tick += delegate
            {
                _statusHideTimer.Stop();
                HideStatus();
            };
            PreviewKeyDown += ShortcutKeyDown;
            Closing += WindowClosing;
        }

        private void GeneratePasswords()
        {
            if (_lowercase.IsChecked != true &&
                _uppercase.IsChecked != true &&
                _numbers.IsChecked != true &&
                _symbols.IsChecked != true)
                return;

            bool oldClipboardReleased = RequestClipboardRelease();
            DropRows();

            try
            {
                IList<string> passwords = PasswordGenerator.GenerateMany(
                    ReadCount(),
                    ReadLength(),
                    _lowercase.IsChecked == true,
                    _uppercase.IsChecked == true,
                    _numbers.IsChecked == true,
                    _symbols.IsChecked == true,
                    _avoidAmbiguous.IsChecked == true);

                for (int i = 0; i < passwords.Count; i++)
                    AddPasswordRow(passwords[i], i + 1);

                RefreshRows();
                ShowResults();
                if (oldClipboardReleased)
                    HideStatus();
                else
                    SetStatus("WAITING TO RELEASE PREVIOUS CLIPBOARD VALUE…", true, 0);
            }
            catch (Exception exception)
            {
                CollapseResults();
                SetStatus(exception.Message.ToUpperInvariant(), true, 2500);
            }
        }

        private void AddPasswordRow(string secret, int number)
        {
            PasswordRow row = new PasswordRow();
            row.Secret = secret;
            row.Number = number;

            Border container = new Border();
            container.MinHeight = 34;
            container.Padding = new Thickness(0, 4, 0, 4);
            container.BorderThickness = new Thickness(0, 0, 0, 1);
            container.SetResourceReference(Border.BorderBrushProperty, Theme.RowEdgeKey);
            AutomationProperties.SetName(container, "Password " + number);
            row.Container = container;

            Grid layout = new Grid();
            layout.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(26) });
            layout.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            layout.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            container.Child = layout;

            TextBlock index = Text(number.ToString("00"), 10.5, FontWeights.Normal, MutedBrush());
            index.SetResourceReference(TextBlock.ForegroundProperty, Theme.RowIndexKey);
            index.FontFamily = Theme.MonoFont;
            index.TextAlignment = TextAlignment.Right;
            index.VerticalAlignment = VerticalAlignment.Center;
            index.Margin = new Thickness(0, 0, 8, 0);
            layout.Children.Add(index);

            TextBlock display = Text("", PasswordFontSize(secret.Length), FontWeights.Normal, InkBrush());
            display.FontFamily = Theme.MonoFont;
            display.VerticalAlignment = VerticalAlignment.Center;
            display.Margin = new Thickness(0, 0, 8, 0);
            display.Cursor = Cursors.Hand;
            if (PasswordFontSize(secret.Length) < 9)
            {
                display.FontSize = 11;
                display.TextWrapping = TextWrapping.Wrap;
            }
            else
            {
                display.TextWrapping = TextWrapping.NoWrap;
            }
            PasswordRow captured = row;
            display.MouseLeftButtonDown += delegate
            {
                if (_mask.IsChecked == true && _active)
                {
                    captured.Revealed = !captured.Revealed;
                    RefreshRow(captured);
                }
            };
            row.Display = display;
            Grid.SetColumn(display, 1);
            layout.Children.Add(display);

            Button copy = new Button();
            copy.Content = "COPY";
            copy.Height = 24;
            copy.MinWidth = 52;
            copy.FontSize = 10;
            AutomationProperties.SetName(copy, "Copy password " + number);
            Apply(copy, Styles.GhostButton);
            copy.Click += delegate { CopyPassword(captured); };
            row.Copy = copy;
            Grid.SetColumn(copy, 2);
            layout.Children.Add(copy);

            _rows.Add(row);
            _resultsPanel.Children.Add(container);
        }

        private static double PasswordFontSize(int length)
        {
            return Math.Min(12, (280.0 / length - 0.6) / 0.62);
        }

        private void CopyPassword(PasswordRow row)
        {
            if (_clipboard == null || !_clipboard.TryCopy(row.Secret))
            {
                SetStatus("CLIPBOARD BUSY — COPY FAILED", true, 2500);
                return;
            }

            foreach (PasswordRow item in _rows)
                item.Copy.Content = "COPY";
            row.Copy.Content = "COPIED";
            _clearRequested = false;
            _clipboardAge.Restart();
            _clipboardTimer.Start();
            SetCopyStatus(row.Number, _clipboardSeconds);
        }

        private void SetCopyStatus(int number, int seconds)
        {
            SetStatus("▮ COPIED " + number.ToString("00") + " · CLIPBOARD RELEASES IN " + seconds + " S", false, 0);
        }

        private void ClearAll()
        {
            bool released = RequestClipboardRelease();
            CollapseResults();
            if (!released)
                SetStatus("CLEARED · WAITING FOR CLIPBOARD", true, 0);
            else
                HideStatus();
        }

        private bool RequestClipboardRelease()
        {
            _clipboardAge.Reset();
            if (_clipboard == null || _clipboard.TryClearOwned())
            {
                _clearRequested = false;
                _clipboardTimer.Stop();
                return true;
            }

            _clearRequested = true;
            _clipboardTimer.Start();
            return false;
        }

        private void ClipboardTick(object sender, EventArgs eventArgs)
        {
            if (_pendingClose)
            {
                if (_clipboard == null || _clipboard.TryClearOwnedOnce())
                {
                    _pendingClose = false;
                    _allowClose = true;
                    _clipboardTimer.Stop();
                    Dispatcher.BeginInvoke(DispatcherPriority.Send, new Action(Close));
                }
                return;
            }

            if (_clearRequested)
            {
                if (_clipboard == null || _clipboard.TryClearOwnedOnce())
                {
                    _clearRequested = false;
                    _clipboardTimer.Stop();
                    HideStatus();
                }
                else
                {
                    SetStatus("WAITING FOR CLIPBOARD…", true, 0);
                }
                return;
            }

            int seconds = (int)Math.Ceiling(_clipboardSeconds - _clipboardAge.Elapsed.TotalSeconds);
            if (seconds > 0)
            {
                PasswordRow copied = null;
                foreach (PasswordRow row in _rows)
                    if ((string)row.Copy.Content == "COPIED") copied = row;
                if (copied != null)
                    SetCopyStatus(copied.Number, seconds);
                return;
            }

            _clipboardAge.Stop();
            if (_clipboard == null || _clipboard.TryClearOwnedOnce())
            {
                _clipboardTimer.Stop();
                foreach (PasswordRow row in _rows)
                    row.Copy.Content = "COPY";
                HideStatus();
            }
            else
            {
                _clearRequested = true;
                SetStatus("WAITING FOR CLIPBOARD…", true, 0);
            }
        }

        private void ShowResults()
        {
            _resultsHost.Visibility = Visibility.Visible;
            _clear.IsEnabled = true;
        }

        private void CollapseResults()
        {
            RefreshRows(false);
            DropRows();
            _resultsHost.Visibility = Visibility.Collapsed;
            _clear.IsEnabled = false;
        }

        private void DropRows()
        {
            foreach (PasswordRow row in _rows)
            {
                row.Display.Text = "";
                row.Secret = null;
            }
            _resultsPanel.Children.Clear();
            _rows.Clear();
        }

        private void SetMask(bool masked)
        {
            foreach (PasswordRow row in _rows)
                row.Revealed = false;
            _mask.ToolTip = masked ? "Show passwords" : "Mask passwords";
            RefreshRows();
        }

        private void RefreshRows()
        {
            RefreshRows(_active);
        }

        private void RefreshRows(bool active)
        {
            foreach (PasswordRow row in _rows)
                RefreshRow(row, active);
        }

        private void RefreshRow(PasswordRow row)
        {
            RefreshRow(row, _active);
        }

        private void RefreshRow(PasswordRow row, bool active)
        {
            bool hidden = !active || (_mask.IsChecked == true && !row.Revealed);
            row.Display.Text = hidden ? new string('•', row.Secret.Length) : row.Secret;
            AutomationProperties.SetName(row.Display, "Password " + row.Number + (hidden ? ", hidden" : ", visible"));
        }

        private void ToggleTheme()
        {
            if (Theme.HighContrast)
                return;

            Theme.Apply(!Theme.IsLight);
            _length.Style = Styles.SliderStyle();
            _theme.Content = Theme.IsLight ? "◑" : "◐";
            _theme.ToolTip = Theme.IsLight ? "Switch to dark theme" : "Switch to light theme";
            InvalidateVisual();
        }

        private void ScheduleRollUp()
        {
            if (Theme.HighContrast || _infoPopup.IsOpen || _pendingClose)
                return;
            _rollTimer.Stop();
            _rollTimer.Start();
        }

        private void RollUp()
        {
            if (_rolledUp || Theme.HighContrast)
                return;
            RefreshRows(false);
            _rolledUp = true;
            _body.Visibility = Visibility.Collapsed;
            _headerButtons.Visibility = Visibility.Collapsed;
            _collapsedHint.Visibility = Visibility.Visible;
            _shell.Padding = new Thickness(16, 8, 16, 8);
            _shell.SetResourceReference(Border.BackgroundProperty, Theme.RolledShellKey);
            _shell.Opacity = 0.50;
        }

        private void Unroll()
        {
            _rollTimer.Stop();
            if (!_rolledUp)
                return;
            _rolledUp = false;
            _shell.Padding = new Thickness(16, 12, 16, 14);
            _shell.SetResourceReference(Border.BackgroundProperty, Theme.ShellKey);
            _body.Visibility = Visibility.Visible;
            _headerButtons.Visibility = Visibility.Visible;
            _collapsedHint.Visibility = Visibility.Collapsed;
            RefreshRows();
        }
        private void ChangeClipboardSeconds(int change)
        {
            _clipboardSeconds = Math.Max(5, Math.Min(60, _clipboardSeconds + change));
            _clipboardSecondsText.Text = _clipboardSeconds + " S";
            _clipMinus.IsEnabled = _clipboardSeconds > 5;
            _clipPlus.IsEnabled = _clipboardSeconds < 60;
        }

        private void SetStatus(string text, bool warning, int autoHideMilliseconds)
        {
            _statusHideTimer.Stop();
            _status.Text = text;
            if (Theme.HighContrast)
                _status.Foreground = warning ? SystemColors.WindowTextBrush : SystemColors.GrayTextBrush;
            else
                _status.SetResourceReference(TextBlock.ForegroundProperty, warning ? Theme.WarningKey : Theme.WarningKey);
            _status.Visibility = Visibility.Visible;
            AutomationProperties.SetName(_status, "Status: " + text);

            if (autoHideMilliseconds > 0)
            {
                _statusHideTimer.Interval = TimeSpan.FromMilliseconds(autoHideMilliseconds);
                _statusHideTimer.Start();
            }
        }

        private void HideStatus()
        {
            _statusHideTimer.Stop();
            _status.Text = "";
            _status.Visibility = Visibility.Collapsed;
        }

        private int ReadLength()
        {
            int length;
            if (!Int32.TryParse(_lengthValue.Text, out length))
                length = 20;
            length = Math.Max(4, Math.Min(64, length));
            _length.Value = length;
            _lengthValue.Text = length.ToString();
            return length;
        }

        private int ReadCount()
        {
            int count;
            if (!Int32.TryParse(_count.Text, out count))
                count = 1;
            count = Math.Max(1, Math.Min(99, count));
            _count.Text = count.ToString();
            return count;
        }

        private void NumericPreviewTextInput(object sender, TextCompositionEventArgs eventArgs)
        {
            eventArgs.Handled = !IsDigits(eventArgs.Text);
        }

        private void NumericPaste(object sender, DataObjectPastingEventArgs eventArgs)
        {
            string text = eventArgs.DataObject.GetData(typeof(string)) as string;
            if (!IsDigits(text))
                eventArgs.CancelCommand();
        }

        private void NumericKeyDown(object sender, KeyEventArgs eventArgs)
        {
            TextBox input = sender as TextBox;
            if (input == null)
                return;
            if (eventArgs.Key == Key.Enter)
            {
                Keyboard.ClearFocus();
                eventArgs.Handled = true;
                return;
            }
            if (eventArgs.Key != Key.Up && eventArgs.Key != Key.Down)
                return;

            int value;
            if (!Int32.TryParse(input.Text, out value))
                value = input == _lengthValue ? 20 : 1;
            value += eventArgs.Key == Key.Up ? 1 : -1;
            if (input == _lengthValue)
                value = Math.Max(4, Math.Min(64, value));
            else
                value = Math.Max(1, Math.Min(99, value));
            input.Text = value.ToString();
            input.SelectAll();
            if (input == _lengthValue)
                _length.Value = value;
            eventArgs.Handled = true;
        }

        private static bool IsDigits(string text)
        {
            if (String.IsNullOrEmpty(text))
                return false;
            foreach (char character in text)
                if (!Char.IsDigit(character)) return false;
            return true;
        }

        private void ShortcutKeyDown(object sender, KeyEventArgs eventArgs)
        {
            if ((Keyboard.Modifiers & ModifierKeys.Control) == 0)
                return;

            if (eventArgs.Key == Key.G)
            {
                GeneratePasswords();
                eventArgs.Handled = true;
            }
            else if (eventArgs.Key == Key.L)
            {
                ClearAll();
                eventArgs.Handled = true;
            }
        }

        private void DragHeader(object sender, MouseButtonEventArgs eventArgs)
        {
            if (_rolledUp)
            {
                Unroll();
                eventArgs.Handled = true;
                return;
            }

            DependencyObject source = eventArgs.OriginalSource as DependencyObject;
            while (source != null)
            {
                if (source is ButtonBase)
                    return;
                source = VisualTreeHelper.GetParent(source);
            }

            if (eventArgs.LeftButton == MouseButtonState.Pressed)
            {
                try { DragMove(); }
                catch (InvalidOperationException) { }
            }
        }

        private void WindowClosing(object sender, CancelEventArgs eventArgs)
        {
            _infoPopup.IsOpen = false;
            _rollTimer.Stop();
            RefreshRows(false);
            DropRows();

            if (_allowClose || _clipboard == null || _clipboard.TryClearOwned())
            {
                _clipboardTimer.Stop();
                _statusHideTimer.Stop();
                return;
            }

            eventArgs.Cancel = true;
            _pendingClose = true;
            _clearRequested = true;
            Hide();
            _clipboardTimer.Start();
        }

        internal void OnSessionEnding(object sender, SessionEndingCancelEventArgs eventArgs)
        {
            _infoPopup.IsOpen = false;
            _rollTimer.Stop();
            RefreshRows(false);
            DropRows();
            if (_clipboard != null)
                _clipboard.TryClearOwned();
            _allowClose = true;
            _pendingClose = false;
            _clearRequested = false;
            _clipboardTimer.Stop();
            _statusHideTimer.Stop();
        }
    }
}