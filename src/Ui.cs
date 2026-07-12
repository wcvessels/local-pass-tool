using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Data;
using System.Windows.Markup;
using System.Windows.Media;
using System.Windows.Media.Effects;

namespace LocalPass
{
    internal static class Theme
    {
        internal static readonly bool HighContrast = SystemParameters.HighContrast;
        internal static readonly FontFamily UiFont = new FontFamily("Segoe UI");
        internal static readonly FontFamily MonoFont = new FontFamily("Cascadia Mono");

        internal const string InkKey = "LocalPass.Ink";
        internal const string MutedKey = "LocalPass.Muted";
        internal const string FocusKey = "LocalPass.Focus";
        internal const string PrimaryTextKey = "LocalPass.PrimaryText";
        internal const string WarningKey = "LocalPass.Warning";
        internal const string BubbleEdgeKey = "LocalPass.ControlEdge";
        internal const string ShellEdgeKey = "LocalPass.ShellEdge";
        internal const string DividerKey = "LocalPass.Divider";
        internal const string RowEdgeKey = "LocalPass.RowEdge";
        internal const string SecondaryTextKey = "LocalPass.SecondaryText";
        internal const string SegmentOffKey = "LocalPass.SegmentOff";
        internal const string RowIndexKey = "LocalPass.RowIndex";
        internal const string IconHoverKey = "LocalPass.IconHover";
        internal const string TrackKey = "LocalPass.Track";
        internal const string ShellKey = "LocalPass.Shell";
        internal const string InfoShellKey = "LocalPass.InfoShell";
        internal const string RolledShellKey = "LocalPass.RolledShell";
        internal const string BubbleFillKey = "LocalPass.ControlFill";
        internal const string BubbleHoverKey = "LocalPass.ControlHover";
        internal const string BubblePressedKey = "LocalPass.ControlPressed";
        internal const string SelectedFillKey = "LocalPass.SelectedFill";
        internal const string PrimaryFillKey = "LocalPass.PrimaryFill";
        internal const string PrimaryHoverKey = "LocalPass.PrimaryHover";
        internal const string PrimaryPressedKey = "LocalPass.PrimaryPressed";

        internal static readonly ResourceDictionary Resources = new ResourceDictionary();
        internal static readonly Brush Transparent = Brushes.Transparent;

        internal static SolidColorBrush Ink { get { return SolidAt(InkKey); } }
        internal static SolidColorBrush Muted { get { return SolidAt(MutedKey); } }
        internal static SolidColorBrush Focus { get { return SolidAt(FocusKey); } }
        internal static SolidColorBrush Track { get { return SolidAt(TrackKey); } }
        internal static bool IsLight { get; private set; }

        static Theme()
        {
            Apply(false);
        }

        internal static void Apply(bool light)
        {
            IsLight = light;
            if (light)
            {
                Resources[InkKey] = Solid(255, 31, 34, 38);
                Resources[MutedKey] = Solid(255, 79, 86, 94);
                Resources[FocusKey] = Solid(140, 31, 34, 38);
                Resources[PrimaryTextKey] = Solid(255, 247, 248, 249);
                Resources[WarningKey] = Solid(255, 132, 83, 24);
                Resources[BubbleEdgeKey] = Solid(64, 31, 34, 38);
                Resources[ShellEdgeKey] = Solid(166, 255, 255, 255);
                Resources[DividerKey] = Solid(31, 31, 34, 38);
                Resources[RowEdgeKey] = Solid(20, 31, 34, 38);
                Resources[SecondaryTextKey] = Solid(255, 43, 47, 52);
                Resources[SegmentOffKey] = Solid(255, 138, 144, 152);
                Resources[RowIndexKey] = Solid(255, 106, 112, 120);
                Resources[IconHoverKey] = Solid(26, 31, 34, 38);
                Resources[TrackKey] = Solid(56, 31, 34, 38);
                Resources[ShellKey] = Gradient(Color.FromArgb(115, 255, 255, 255), Color.FromArgb(140, 236, 240, 244));
                Resources[InfoShellKey] = Gradient(Color.FromArgb(122, 255, 255, 255), Color.FromArgb(148, 236, 240, 244));
                Resources[RolledShellKey] = Gradient(Color.FromArgb(173, 255, 255, 255), Color.FromArgb(199, 236, 240, 244));
                Resources[BubbleFillKey] = Gradient(Color.FromArgb(128, 255, 255, 255), Color.FromArgb(128, 255, 255, 255));
                Resources[BubbleHoverKey] = Gradient(Color.FromArgb(217, 255, 255, 255), Color.FromArgb(217, 255, 255, 255));
                Resources[BubblePressedKey] = Gradient(Color.FromArgb(166, 255, 255, 255), Color.FromArgb(166, 255, 255, 255));
                Resources[SelectedFillKey] = Gradient(Color.FromArgb(31, 31, 34, 38), Color.FromArgb(31, 31, 34, 38));
                Resources[PrimaryFillKey] = Gradient(Color.FromArgb(255, 35, 38, 43), Color.FromArgb(255, 35, 38, 43));
                Resources[PrimaryHoverKey] = Gradient(Color.FromArgb(255, 16, 18, 21), Color.FromArgb(255, 16, 18, 21));
                Resources[PrimaryPressedKey] = Gradient(Color.FromArgb(255, 22, 24, 28), Color.FromArgb(255, 22, 24, 28));
            }
            else
            {
                Resources[InkKey] = Solid(255, 242, 243, 245);
                Resources[MutedKey] = Solid(255, 178, 184, 190);
                Resources[FocusKey] = Solid(128, 255, 255, 255);
                Resources[PrimaryTextKey] = Solid(255, 22, 24, 28);
                Resources[WarningKey] = Solid(255, 229, 171, 102);
                Resources[BubbleEdgeKey] = Solid(46, 255, 255, 255);
                Resources[ShellEdgeKey] = Solid(41, 255, 255, 255);
                Resources[DividerKey] = Solid(23, 255, 255, 255);
                Resources[RowEdgeKey] = Solid(15, 255, 255, 255);
                Resources[SecondaryTextKey] = Solid(255, 232, 235, 238);
                Resources[SegmentOffKey] = Solid(255, 121, 129, 138);
                Resources[RowIndexKey] = Solid(255, 138, 145, 153);
                Resources[IconHoverKey] = Solid(26, 255, 255, 255);
                Resources[TrackKey] = Solid(46, 255, 255, 255);
                Resources[ShellKey] = Gradient(Color.FromArgb(133, 40, 45, 52), Color.FromArgb(158, 17, 20, 24));
                Resources[InfoShellKey] = Gradient(Color.FromArgb(140, 40, 45, 52), Color.FromArgb(166, 17, 20, 24));
                Resources[RolledShellKey] = Gradient(Color.FromArgb(184, 40, 45, 52), Color.FromArgb(214, 17, 20, 24));
                Resources[BubbleFillKey] = Gradient(Color.FromArgb(18, 255, 255, 255), Color.FromArgb(18, 255, 255, 255));
                Resources[BubbleHoverKey] = Gradient(Color.FromArgb(46, 255, 255, 255), Color.FromArgb(46, 255, 255, 255));
                Resources[BubblePressedKey] = Gradient(Color.FromArgb(31, 255, 255, 255), Color.FromArgb(31, 255, 255, 255));
                Resources[SelectedFillKey] = Gradient(Color.FromArgb(41, 255, 255, 255), Color.FromArgb(41, 255, 255, 255));
                Resources[PrimaryFillKey] = Gradient(Color.FromArgb(255, 238, 241, 244), Color.FromArgb(255, 238, 241, 244));
                Resources[PrimaryHoverKey] = Gradient(Color.FromArgb(255, 255, 255, 255), Color.FromArgb(255, 255, 255, 255));
                Resources[PrimaryPressedKey] = Gradient(Color.FromArgb(255, 222, 226, 230), Color.FromArgb(255, 222, 226, 230));
            }
        }

        internal static object Dynamic(string key)
        {
            return new DynamicResourceExtension(key);
        }

        private static SolidColorBrush Solid(byte alpha, byte red, byte green, byte blue)
        {
            SolidColorBrush brush = new SolidColorBrush(Color.FromArgb(alpha, red, green, blue));
            brush.Freeze();
            return brush;
        }

        private static LinearGradientBrush Gradient(Color top, Color bottom)
        {
            LinearGradientBrush gradient = new LinearGradientBrush();
            gradient.StartPoint = new Point(0, 0);
            gradient.EndPoint = new Point(0, 1);
            gradient.GradientStops.Add(new GradientStop(top, 0));
            gradient.GradientStops.Add(new GradientStop(bottom, 1));
            gradient.Freeze();
            return gradient;
        }

        private static SolidColorBrush SolidAt(string key)
        {
            return (SolidColorBrush)Resources[key];
        }
    }

    internal static class Styles
    {
        internal static readonly Style PrimaryButton = CreatePrimaryButton();
        internal static readonly Style GhostButton = CreateGhostButton();
        internal static readonly Style IconButton = CreateIconButton(typeof(Button));
        internal static readonly Style IconToggle = CreateIconButton(typeof(ToggleButton));
        internal static readonly Style SegmentToggle = CreateSegmentToggle();
        internal static readonly Style Input = CreateInput();
        internal static readonly Style ThinScrollBar = CreateThinScrollBar();

        internal static Style SliderStyle()
        {
            string fill = Theme.Ink.Color.ToString();
            string track = Theme.Track.Color.ToString();
            string xaml =
                "<Style xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation' " +
                "xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' TargetType='{x:Type Slider}'>" +
                "<Setter Property='Cursor' Value='Hand'/>" +
                "<Setter Property='Template'><Setter.Value>" +
                "<ControlTemplate TargetType='{x:Type Slider}'>" +
                "<Grid Height='16' Background='Transparent'>" +
                "<Track x:Name='PART_Track' Margin='5,0' VerticalAlignment='Center' " +
                "Minimum='{TemplateBinding Minimum}' Maximum='{TemplateBinding Maximum}' " +
                "Value='{TemplateBinding Value}' Orientation='{TemplateBinding Orientation}' " +
                "IsDirectionReversed='{TemplateBinding IsDirectionReversed}'>" +
                "<Track.DecreaseRepeatButton><RepeatButton Command='{x:Static Slider.DecreaseLarge}' Focusable='False'>" +
                "<RepeatButton.Template><ControlTemplate TargetType='{x:Type RepeatButton}'><Border Height='2' Background='" + fill + "'/></ControlTemplate></RepeatButton.Template>" +
                "</RepeatButton></Track.DecreaseRepeatButton>" +
                "<Track.IncreaseRepeatButton><RepeatButton Command='{x:Static Slider.IncreaseLarge}' Focusable='False'>" +
                "<RepeatButton.Template><ControlTemplate TargetType='{x:Type RepeatButton}'><Border Height='2' Background='" + track + "'/></ControlTemplate></RepeatButton.Template>" +
                "</RepeatButton></Track.IncreaseRepeatButton>" +
                "<Track.Thumb><Thumb Width='10' Height='10' Focusable='False'>" +
                "<Thumb.Template><ControlTemplate TargetType='{x:Type Thumb}'><Border CornerRadius='2' Background='" + fill + "'>" +
                "<Border.Effect><DropShadowEffect BlurRadius='3' ShadowDepth='1' Opacity='.35'/></Border.Effect>" +
                "</Border></ControlTemplate></Thumb.Template></Thumb></Track.Thumb>" +
                "</Track></Grid></ControlTemplate></Setter.Value></Setter></Style>";
            return (Style)XamlReader.Parse(xaml);
        }

        private static Style CreatePrimaryButton()
        {
            Style style = ButtonBaseStyle(typeof(Button), 4);
            style.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.PrimaryFillKey)));
            style.Setters.Add(new Setter(Control.ForegroundProperty, Theme.Dynamic(Theme.PrimaryTextKey)));
            style.Setters.Add(new Setter(Control.BorderThicknessProperty, new Thickness(0)));
            style.Setters.Add(new Setter(Control.FontWeightProperty, FontWeights.Bold));
            style.Setters.Add(new Setter(UIElement.EffectProperty, Shadow()));
            AddBackgroundState(style, Theme.PrimaryHoverKey, Theme.PrimaryPressedKey, true);
            AddDisabled(style);
            return style;
        }

        private static Style CreateGhostButton()
        {
            Style style = ButtonBaseStyle(typeof(Button), 4);
            style.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.BubbleFillKey)));
            style.Setters.Add(new Setter(Control.ForegroundProperty, Theme.Dynamic(Theme.SecondaryTextKey)));
            style.Setters.Add(new Setter(Control.BorderBrushProperty, Theme.Dynamic(Theme.BubbleEdgeKey)));
            style.Setters.Add(new Setter(Control.BorderThicknessProperty, new Thickness(1)));
            AddBackgroundState(style, Theme.BubbleHoverKey, Theme.BubblePressedKey, false);
            AddDisabled(style);
            return style;
        }

        private static Style CreateIconButton(Type type)
        {
            Style style = ButtonBaseStyle(type, 5);
            style.Setters.Add(new Setter(Control.BackgroundProperty, Brushes.Transparent));
            style.Setters.Add(new Setter(Control.ForegroundProperty, Theme.Dynamic(Theme.MutedKey)));
            style.Setters.Add(new Setter(Control.BorderThicknessProperty, new Thickness(0)));
            style.Setters.Add(new Setter(Control.PaddingProperty, new Thickness(0)));

            Trigger hover = Trigger(UIElement.IsMouseOverProperty, true);
            hover.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.IconHoverKey)));
            hover.Setters.Add(new Setter(Control.ForegroundProperty, Theme.Dynamic(Theme.InkKey)));
            style.Triggers.Add(hover);

            Trigger pressed = Trigger(ButtonBase.IsPressedProperty, true);
            pressed.Setters.Add(new Setter(UIElement.OpacityProperty, 0.72));
            style.Triggers.Add(pressed);

            if (type == typeof(ToggleButton))
            {
                Trigger selected = Trigger(ToggleButton.IsCheckedProperty, true);
                selected.Setters.Add(new Setter(Control.ForegroundProperty, Theme.Dynamic(Theme.InkKey)));
                style.Triggers.Add(selected);
            }
            AddDisabled(style);
            return style;
        }

        private static Style CreateSegmentToggle()
        {
            Style style = ButtonBaseStyle(typeof(ToggleButton), 0);
            style.Setters.Add(new Setter(Control.BackgroundProperty, Brushes.Transparent));
            style.Setters.Add(new Setter(Control.ForegroundProperty, Theme.Dynamic(Theme.SegmentOffKey)));
            style.Setters.Add(new Setter(Control.BorderBrushProperty, Theme.Dynamic(Theme.BubbleEdgeKey)));
            style.Setters.Add(new Setter(Control.BorderThicknessProperty, new Thickness(0)));
            style.Setters.Add(new Setter(Control.PaddingProperty, new Thickness(4, 0, 4, 0)));

            Trigger hover = Trigger(UIElement.IsMouseOverProperty, true);
            hover.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.IconHoverKey)));
            style.Triggers.Add(hover);

            Trigger selected = Trigger(ToggleButton.IsCheckedProperty, true);
            selected.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.SelectedFillKey)));
            selected.Setters.Add(new Setter(Control.ForegroundProperty, Theme.Dynamic(Theme.InkKey)));
            style.Triggers.Add(selected);
            AddDisabled(style);
            return style;
        }

        private static Style CreateInput()
        {
            Style style = new Style(typeof(TextBox));
            style.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.BubbleFillKey)));
            style.Setters.Add(new Setter(Control.ForegroundProperty, Theme.Dynamic(Theme.InkKey)));
            style.Setters.Add(new Setter(Control.BorderBrushProperty, Theme.Dynamic(Theme.BubbleEdgeKey)));
            style.Setters.Add(new Setter(Control.BorderThicknessProperty, new Thickness(1)));
            style.Setters.Add(new Setter(Control.PaddingProperty, new Thickness(0)));
            style.Setters.Add(new Setter(Control.VerticalContentAlignmentProperty, VerticalAlignment.Center));
            style.Setters.Add(new Setter(Control.HorizontalContentAlignmentProperty, HorizontalAlignment.Center));
            style.Setters.Add(new Setter(Control.FontWeightProperty, FontWeights.Normal));
            style.Setters.Add(new Setter(Control.TemplateProperty, InputTemplate()));
            return style;
        }

        private static Style CreateThinScrollBar()
        {
            Style style = new Style(typeof(ScrollBar));
            style.Setters.Add(new Setter(FrameworkElement.WidthProperty, 6.0));
            style.Setters.Add(new Setter(Control.BackgroundProperty, Brushes.Transparent));
            style.Setters.Add(new Setter(UIElement.OpacityProperty, 0.55));
            return style;
        }

        private static Style ButtonBaseStyle(Type type, double radius)
        {
            Style style = new Style(type);
            style.Setters.Add(new Setter(Control.FontFamilyProperty, Theme.UiFont));
            style.Setters.Add(new Setter(Control.FontSizeProperty, 11.0));
            style.Setters.Add(new Setter(Control.CursorProperty, System.Windows.Input.Cursors.Hand));
            style.Setters.Add(new Setter(Control.HorizontalContentAlignmentProperty, HorizontalAlignment.Center));
            style.Setters.Add(new Setter(Control.VerticalContentAlignmentProperty, VerticalAlignment.Center));
            style.Setters.Add(new Setter(UIElement.RenderTransformOriginProperty, new Point(0.5, 0.5)));
            style.Setters.Add(new Setter(Control.TemplateProperty, ButtonTemplate(type, radius)));
            return style;
        }

        private static ControlTemplate ButtonTemplate(Type type, double radius)
        {
            ControlTemplate template = new ControlTemplate(type);
            FrameworkElementFactory body = new FrameworkElementFactory(typeof(Border), "body");
            body.SetValue(Border.CornerRadiusProperty, new CornerRadius(radius));
            body.SetBinding(Border.BackgroundProperty, ParentBinding("Background"));
            body.SetBinding(Border.BorderBrushProperty, ParentBinding("BorderBrush"));
            body.SetBinding(Border.BorderThicknessProperty, ParentBinding("BorderThickness"));
            body.SetBinding(Border.PaddingProperty, ParentBinding("Padding"));

            FrameworkElementFactory content = new FrameworkElementFactory(typeof(ContentPresenter));
            content.SetValue(ContentPresenter.RecognizesAccessKeyProperty, true);
            content.SetBinding(ContentPresenter.ContentProperty, ParentBinding("Content"));
            content.SetBinding(ContentPresenter.ContentTemplateProperty, ParentBinding("ContentTemplate"));
            content.SetBinding(ContentPresenter.HorizontalAlignmentProperty, ParentBinding("HorizontalContentAlignment"));
            content.SetBinding(ContentPresenter.VerticalAlignmentProperty, ParentBinding("VerticalContentAlignment"));
            body.AppendChild(content);
            template.VisualTree = body;

            Trigger focused = Trigger(UIElement.IsKeyboardFocusedProperty, true);
            focused.Setters.Add(new Setter(Border.BorderBrushProperty, Theme.Dynamic(Theme.FocusKey), "body"));
            template.Triggers.Add(focused);
            return template;
        }

        private static ControlTemplate InputTemplate()
        {
            ControlTemplate template = new ControlTemplate(typeof(TextBox));
            FrameworkElementFactory body = new FrameworkElementFactory(typeof(Border), "body");
            body.SetValue(Border.CornerRadiusProperty, new CornerRadius(4));
            body.SetBinding(Border.BackgroundProperty, ParentBinding("Background"));
            body.SetBinding(Border.BorderBrushProperty, ParentBinding("BorderBrush"));
            body.SetBinding(Border.BorderThicknessProperty, ParentBinding("BorderThickness"));
            FrameworkElementFactory host = new FrameworkElementFactory(typeof(ScrollViewer), "PART_ContentHost");
            host.SetBinding(Control.PaddingProperty, ParentBinding("Padding"));
            body.AppendChild(host);
            template.VisualTree = body;

            Trigger focused = Trigger(UIElement.IsKeyboardFocusedProperty, true);
            focused.Setters.Add(new Setter(Border.BorderBrushProperty, Theme.Dynamic(Theme.FocusKey), "body"));
            template.Triggers.Add(focused);
            return template;
        }

        private static void AddBackgroundState(Style style, string hoverKey, string pressedKey, bool movePressed)
        {
            Trigger hover = Trigger(UIElement.IsMouseOverProperty, true);
            hover.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(hoverKey)));
            style.Triggers.Add(hover);
            Trigger pressed = Trigger(ButtonBase.IsPressedProperty, true);
            pressed.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(pressedKey)));
            if (movePressed)
                pressed.Setters.Add(new Setter(UIElement.RenderTransformProperty, new TranslateTransform(0, 1)));
            style.Triggers.Add(pressed);
        }

        private static Binding ParentBinding(string path)
        {
            Binding binding = new Binding(path);
            binding.RelativeSource = new RelativeSource(RelativeSourceMode.TemplatedParent);
            return binding;
        }

        private static Trigger Trigger(DependencyProperty property, object value)
        {
            Trigger trigger = new Trigger();
            trigger.Property = property;
            trigger.Value = value;
            return trigger;
        }

        private static DropShadowEffect Shadow()
        {
            DropShadowEffect shadow = new DropShadowEffect();
            shadow.BlurRadius = 6;
            shadow.ShadowDepth = 2;
            shadow.Opacity = 0.30;
            shadow.Color = Colors.Black;
            shadow.Freeze();
            return shadow;
        }

        private static void AddDisabled(Style style)
        {
            Trigger disabled = Trigger(UIElement.IsEnabledProperty, false);
            disabled.Setters.Add(new Setter(UIElement.OpacityProperty, 0.38));
            disabled.Setters.Add(new Setter(Control.CursorProperty, System.Windows.Input.Cursors.Arrow));
            style.Triggers.Add(disabled);
        }
    }
}