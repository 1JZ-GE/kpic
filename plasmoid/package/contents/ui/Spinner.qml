import QtQuick
import org.kde.kirigami as Kirigami

// indeterminate dots spinner, rotates while running
Item {
    id: root
    property int radius: 10
    property color color: Kirigami.Theme.textColor
    property alias running: timer.running

    property int _innerRadius: radius * 0.7
    property int _currentIndex: 0

    width: radius * 2
    height: radius * 2

    Repeater {
        id: repeater
        model: 8
        delegate: Component {
            Rectangle {
                property int _rotation: (360 / repeater.model) * index
                property int _maxIndex: root._currentIndex + 1
                property int _minIndex: root._currentIndex - 1

                width: root.width - (root._innerRadius * 2)
                height: width * 0.5
                x: _getPosOnCircle(_rotation).x
                y: _getPosOnCircle(_rotation).y
                color: root.color
                opacity: (index >= _minIndex && index <= _maxIndex) || (index === 0 && root._currentIndex + 1 > 7) ? 1 : 0.3
                transform: Rotation {
                    angle: 360 - _rotation
                    origin {
                        x: 0
                        y: height / 2
                    }
                }

                Behavior on opacity { NumberAnimation { duration: 200 } }
            }
        }
    }

    Timer {
        id: timer
        interval: 80
        repeat: true
        running: true
        onTriggered: root._currentIndex = root._currentIndex === 7 ? 0 : root._currentIndex + 1
    }

    function _toRadian(degree) {
        return (degree * 3.14159265) / 180.0
    }

    function _getPosOnCircle(angleInDegree) {
        var centerX = root.width / 2
        var centerY = root.height / 2
        var posX = centerX + root._innerRadius * Math.cos(_toRadian(angleInDegree))
        var posY = centerY - root._innerRadius * Math.sin(_toRadian(angleInDegree))
        return Qt.point(posX, posY)
    }
}