// A Qt Widgets window, for the real-app checks of tools/test-vm --desktop.
#include <QApplication>
#include <QLabel>
#include <QPushButton>
#include <QVBoxLayout>
#include <QWidget>

int main(int argc, char *argv[])
{
    QApplication app(argc, argv);
    QGuiApplication::setDesktopFileName(QStringLiteral("dev.frascella.Silica.QtTest"));
    QWidget window;
    window.setWindowTitle(QStringLiteral("Qt test"));
    auto *layout = new QVBoxLayout(&window);
    layout->addWidget(new QLabel(QStringLiteral("Qt %1 on Wayland").arg(QLatin1StringView(qVersion()))));
    auto *quit = new QPushButton(QStringLiteral("Quit"));
    QObject::connect(quit, &QPushButton::clicked, &app, &QApplication::quit);
    layout->addWidget(quit);
    window.resize(480, 320);
    window.show();
    return app.exec();
}
