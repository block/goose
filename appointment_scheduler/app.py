from flask import Flask, request, jsonify
from models import Appointment
from database import db_session, init_db

app = Flask(__name__)

@app.route('/schedule', methods=['POST'])
def schedule_appointment():
    data = request.get_json()
    name = data['name']
    mobile_phone = data['mobile_phone']
    date = data['date']

    appointment = Appointment(name=name, mobile_phone=mobile_phone, date=date)
    db_session.add(appointment)
    db_session.commit()

    return jsonify({'message': 'Appointment scheduled successfully'})

if __name__ == '__main__':
    init_db()
    app.run(debug=True)